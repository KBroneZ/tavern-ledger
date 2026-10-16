//! Is the game set up to write a complete `Power.log`? Two files decide it
//! (T-106): `log.config` must enable the Power log, and `client.config` next
//! to `Hearthstone.exe` must lift the log size limit (without that the game
//! stops writing at about 10 MB). Read-only: this module never writes either
//! file, it only says what to change (D-004).

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::discover::Resolution;
use crate::setup_fix::log_config_target;

pub const LOG_CONFIG_FILE: &str = "log.config";
pub const CLIENT_CONFIG_FILE: &str = "client.config";

const POWER_SECTION: &str = "Power";
const REQUIRED_POWER: [(&str, &str); 3] = [
    ("LogLevel", "1"),
    ("FilePrinting", "true"),
    ("Verbose", "true"),
];
/// Optional (T-314): the scene changes the app reads to say "in the
/// Battlegrounds lobby" or "game loading" and to end a game that was left.
const LOADING_SCREEN_SECTION: &str = "LoadingScreen";
const OPTIONAL_LOADING_SCREEN: [(&str, &str); 2] = [("LogLevel", "1"), ("FilePrinting", "true")];
const LOG_SECTION: &str = "Log";
const REQUIRED_LOG: [(&str, &str); 1] = [("FileSizeLimit.Int", "-1")];

/// Result for one config file. Three different problems, three states.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileCheck {
    Ok,
    /// The file (or the folder it lives in) does not exist.
    FileMissing,
    /// The app does not know where the file is (no Hearthstone folder in use,
    /// no local data folder), so it cannot say anything about it. Not "ok",
    /// not "missing".
    CannotCheck,
    /// The file exists but could not be read (the error kind).
    Unreadable(String),
    /// The file was read but lacks these `Key=value` lines, or has the keys
    /// with another value.
    SettingsMissing(Vec<String>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetupReport {
    pub log_config: FileCheck,
    pub client_config: FileCheck,
    /// The optional `[LoadingScreen]` section of `log.config` (T-314); not
    /// part of [`SetupReport::is_ok`].
    pub loading_screen: FileCheck,
    log_config_path: Option<PathBuf>,
    client_config_path: Option<PathBuf>,
}

/// `%LOCALAPPDATA%\Blizzard\Hearthstone\log.config`.
pub fn default_log_config_path() -> Option<PathBuf> {
    let local = std::env::var_os("LOCALAPPDATA")?;
    Some(
        PathBuf::from(local)
            .join("Blizzard")
            .join("Hearthstone")
            .join(LOG_CONFIG_FILE),
    )
}

/// `client.config` sits next to `Hearthstone.exe`, one level above `Logs`.
pub fn client_config_path(logs_dir: &Path) -> Option<PathBuf> {
    Some(logs_dir.parent()?.join(CLIENT_CONFIG_FILE))
}

/// The check for the folders the app uses: `local_app_data` is
/// `%LOCALAPPDATA%` (where `log.config` goes) and `install_dir` the Hearthstone
/// install in use (where `client.config` goes). A folder the app does not
/// know makes that file "cannot check", never "missing" or "ok".
pub fn check_install(local_app_data: Option<&Path>, install_dir: Option<&Path>) -> SetupReport {
    let log_config = local_app_data.map(log_config_target);
    let client_config = install_dir.map(|dir| dir.join(CLIENT_CONFIG_FILE));
    check_setup(log_config.as_deref(), client_config.as_deref())
}

pub fn check_setup(log_config: Option<&Path>, client_config: Option<&Path>) -> SetupReport {
    SetupReport {
        log_config: check_file(log_config, POWER_SECTION, &REQUIRED_POWER),
        loading_screen: check_file(log_config, LOADING_SCREEN_SECTION, &OPTIONAL_LOADING_SCREEN),
        client_config: check_file(client_config, LOG_SECTION, &REQUIRED_LOG),
        log_config_path: log_config.map(Path::to_path_buf),
        client_config_path: client_config.map(Path::to_path_buf),
    }
}

fn check_file(path: Option<&Path>, section: &str, required: &[(&str, &str)]) -> FileCheck {
    let Some(path) = path else {
        return FileCheck::CannotCheck;
    };
    match fs::read(path) {
        Ok(bytes) => check_text(&String::from_utf8_lossy(&bytes), section, required),
        Err(e) if e.kind() == io::ErrorKind::NotFound => FileCheck::FileMissing,
        Err(e) => FileCheck::Unreadable(format!("{:?}", e.kind())),
    }
}

fn check_text(text: &str, section: &str, required: &[(&str, &str)]) -> FileCheck {
    let sections = parse_ini(text);
    let values = sections.get(&section.to_lowercase());
    let missing: Vec<String> = required
        .iter()
        .filter(|(key, expected)| {
            let found = values.and_then(|v| v.get(&key.to_lowercase()));
            !found.is_some_and(|value| value.eq_ignore_ascii_case(expected))
        })
        .map(|(key, expected)| format!("{key}={expected}"))
        .collect();
    if missing.is_empty() {
        FileCheck::Ok
    } else {
        FileCheck::SettingsMissing(missing)
    }
}

type Sections = HashMap<String, HashMap<String, String>>;

/// INI-like parser; section and key names are compared in lower case. Keys
/// before any section are ignored.
fn parse_ini(text: &str) -> Sections {
    let mut sections = Sections::new();
    let mut current: Option<String> = None;
    for raw in text.trim_start_matches('\u{feff}').lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with([';', '#']) {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            let name = name.trim().to_lowercase();
            sections.entry(name.clone()).or_default();
            current = Some(name);
        } else if let (Some(name), Some((key, value))) = (&current, line.split_once('=')) {
            if let Some(section) = sections.get_mut(name) {
                section.insert(key.trim().to_lowercase(), value.trim().to_string());
            }
        }
    }
    sections
}

/// A section of a config file and the `Key=value` lines it must hold.
#[derive(Clone, Copy, Debug)]
pub struct Wanted {
    pub section: &'static str,
    pub lines: &'static [(&'static str, &'static str)],
}

/// `log.config`: the Power log (needed).
pub const POWER: Wanted = Wanted {
    section: POWER_SECTION,
    lines: &REQUIRED_POWER,
};
/// `log.config`: the scene changes (optional, T-314).
pub const LOADING_SCREEN: Wanted = Wanted {
    section: LOADING_SCREEN_SECTION,
    lines: &OPTIONAL_LOADING_SCREEN,
};
/// `client.config`: no size limit on the logs (needed).
pub const LOG: Wanted = Wanted {
    section: LOG_SECTION,
    lines: &REQUIRED_LOG,
};

/// Why a file's text is left alone: the app only edits plain text it can read
/// back exactly (UTF-8 with or without a byte order mark, or ASCII).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MergeRefusal {
    NotPlainText,
}

const BOM: &str = "\u{feff}";
const LF: &str = "\n";
const CRLF: &str = "\r\n";

struct Line {
    text: String,
    eol: &'static str,
}

/// What a line is, read the way [`parse_ini`] reads it.
enum Kind {
    Header(String),
    Key {
        section: String,
        key: String,
        value: String,
    },
    Other,
}

/// The text of a config file with the wanted sections and lines in it, or
/// `None` when it already has them. Only missing lines are added (at the end
/// of their section, or a new section at the end of the file) and a key with
/// another value is changed in place (the last one, which the game reads);
/// every other line, comment, space, line end and the byte order mark stay as
/// they were, and so does a missing final newline. D-154.
pub fn merge_config(bytes: &[u8], wanted: &[Wanted]) -> Result<Option<Vec<u8>>, MergeRefusal> {
    let plain = !bytes.starts_with(&[0xFF, 0xFE])
        && !bytes.starts_with(&[0xFE, 0xFF])
        && !bytes.contains(&0);
    let text = std::str::from_utf8(bytes)
        .ok()
        .filter(|_| plain)
        .ok_or(MergeRefusal::NotPlainText)?;
    let (bom, body) = match text.strip_prefix(BOM) {
        Some(rest) => (BOM, rest),
        None => ("", text),
    };
    let mut lines = split_lines(body);
    let eol = majority_eol(&lines);
    let had_final_eol = lines.last().is_none_or(|l| !l.eol.is_empty());
    let mut changed = false;
    for want in wanted {
        for &(key, value) in want.lines {
            changed |= ensure_line(&mut lines, want.section, key, value, eol);
        }
    }
    if !changed {
        return Ok(None);
    }
    if !had_final_eol {
        if let Some(last) = lines.last_mut() {
            last.eol = "";
        }
    }
    let mut out = String::from(bom);
    for line in &lines {
        out.push_str(&line.text);
        out.push_str(line.eol);
    }
    Ok(Some(out.into_bytes()))
}

fn split_lines(body: &str) -> Vec<Line> {
    body.split_inclusive('\n')
        .map(|piece| match piece.strip_suffix(LF) {
            Some(rest) => match rest.strip_suffix('\r') {
                Some(text) => Line {
                    text: text.to_string(),
                    eol: CRLF,
                },
                None => Line {
                    text: rest.to_string(),
                    eol: LF,
                },
            },
            None => Line {
                text: piece.to_string(),
                eol: "",
            },
        })
        .collect()
}

/// The file's usual line end; CRLF (Windows) when there is none or it is a tie.
fn majority_eol(lines: &[Line]) -> &'static str {
    let crlf = lines.iter().filter(|l| l.eol == CRLF).count();
    let lf = lines.iter().filter(|l| l.eol == LF).count();
    if lf > crlf {
        LF
    } else {
        CRLF
    }
}

fn classify(lines: &[Line]) -> Vec<Kind> {
    let mut current: Option<String> = None;
    lines
        .iter()
        .map(|line| {
            let text = line.text.trim();
            if text.is_empty() || text.starts_with([';', '#']) {
                return Kind::Other;
            }
            if let Some(name) = text.strip_prefix('[').and_then(|t| t.strip_suffix(']')) {
                let name = name.trim().to_lowercase();
                current = Some(name.clone());
                return Kind::Header(name);
            }
            match (&current, text.split_once('=')) {
                (Some(section), Some((key, value))) => Kind::Key {
                    section: section.clone(),
                    key: key.trim().to_lowercase(),
                    value: value.trim().to_string(),
                },
                _ => Kind::Other,
            }
        })
        .collect()
}

/// Makes `section` hold `key=value`; whether the lines changed.
fn ensure_line(
    lines: &mut Vec<Line>,
    section: &str,
    key: &str,
    value: &str,
    eol: &'static str,
) -> bool {
    let (section_l, key_l) = (section.to_lowercase(), key.to_lowercase());
    let kinds = classify(lines);
    let last_setting = kinds.iter().rposition(
        |k| matches!(k, Kind::Key { section: s, key: k, .. } if *s == section_l && *k == key_l),
    );
    if let Some(at) = last_setting {
        let Kind::Key { value: found, .. } = &kinds[at] else {
            return false;
        };
        if found.eq_ignore_ascii_case(value) {
            return false;
        }
        lines[at].text = with_value(&lines[at].text, value);
        return true;
    }
    let new_line = Line {
        text: format!("{key}={value}"),
        eol,
    };
    // The end of the section's last block: its header or its last setting.
    let anchor = kinds.iter().rposition(|k| match k {
        Kind::Header(name) => *name == section_l,
        Kind::Key { section: s, .. } => *s == section_l,
        Kind::Other => false,
    });
    match anchor {
        Some(at) => {
            if lines[at].eol.is_empty() {
                lines[at].eol = eol;
            }
            lines.insert(at + 1, new_line);
        }
        None => {
            if let Some(last) = lines.last_mut() {
                if last.eol.is_empty() {
                    last.eol = eol;
                }
                if !last.text.trim().is_empty() {
                    lines.push(Line {
                        text: String::new(),
                        eol,
                    });
                }
            }
            lines.push(Line {
                text: format!("[{section}]"),
                eol,
            });
            lines.push(new_line);
        }
    }
    true
}

/// The line with only its value replaced: the key, the spaces around the
/// `=` and after the value stay.
fn with_value(line: &str, value: &str) -> String {
    let Some((head, after)) = line.split_once('=') else {
        return line.to_string();
    };
    let rest = after.trim_start();
    let lead = &after[..after.len() - rest.len()];
    let tail = &rest[rest.trim_end().len()..];
    format!("{head}={lead}{value}{tail}")
}

impl SetupReport {
    pub fn is_ok(&self) -> bool {
        self.log_config == FileCheck::Ok && self.client_config == FileCheck::Ok
    }

    /// Plain-English lines for the window: what is wrong and what to change.
    /// Empty when the setup is fine.
    pub fn messages(&self) -> Vec<String> {
        let mut out = Vec::new();
        describe(
            &mut out,
            &self.log_config,
            LOG_CONFIG_FILE,
            self.log_config_path.as_deref(),
            POWER_SECTION,
            &REQUIRED_POWER,
            "Without the Power log Tavern Ledger sees no games.",
        );
        describe(
            &mut out,
            &self.client_config,
            CLIENT_CONFIG_FILE,
            self.client_config_path.as_deref(),
            LOG_SECTION,
            &REQUIRED_LOG,
            "Without it the game stops writing Power.log at about 10 MB and later games are lost.",
        );
        out
    }

    /// Optional settings that add to the app but are not needed, in plain
    /// English; empty when they are there or when `log.config` itself needs
    /// fixing first (the messages say that).
    pub fn notes(&self) -> Vec<String> {
        let readable = matches!(
            self.log_config,
            FileCheck::Ok | FileCheck::SettingsMissing(_)
        );
        match &self.loading_screen {
            FileCheck::SettingsMissing(lines) if readable => {
                let place = self
                    .log_config_path
                    .as_deref()
                    .map_or(LOG_CONFIG_FILE.to_string(), |p| p.display().to_string());
                vec![format!(
                    "Optional: {LOG_CONFIG_FILE} ({place}) can have these lines under [{LOADING_SCREEN_SECTION}]: {}. Without them the app reads Power.log only and works as before; with them it shows when you are in the Battlegrounds lobby or a game is loading, and the overlay ends a game you leave before it is over. Restart the game afterwards.",
                    lines.join(", ")
                )]
            }
            _ => Vec::new(),
        }
    }
}

/// What to tell the player when no game folder can be used: nothing found,
/// or the folder they chose cannot be used now. Empty when a folder is in
/// use (the file checks above then say what is wrong inside it).
pub fn folder_messages(resolution: &Resolution) -> Vec<String> {
    match resolution {
        Resolution::Found(_) => Vec::new(),
        Resolution::NotFound => vec![
            "Tavern Ledger could not find Hearthstone on this PC. Choose the folder where the game is installed, or start the game once."
                .to_string(),
        ],
        Resolution::ChosenUnusable { dir, problem } => vec![format!(
            "The Hearthstone folder you chose ({}) cannot be used. {} Choose it again, or go back to finding it automatically.",
            dir.display(),
            problem.message()
        )],
    }
}

fn describe(
    out: &mut Vec<String>,
    check: &FileCheck,
    file: &str,
    path: Option<&Path>,
    section: &str,
    required: &[(&str, &str)],
    why: &str,
) {
    let place = path.map_or(file.to_string(), |p| p.display().to_string());
    let all_lines: Vec<String> = required.iter().map(|(k, v)| format!("{k}={v}")).collect();
    match check {
        FileCheck::Ok => {}
        FileCheck::CannotCheck => out.push(format!(
            "{file} could not be checked: {}.",
            if file == LOG_CONFIG_FILE {
                "this PC's local data folder is not known"
            } else {
                "no Hearthstone folder is in use"
            }
        )),
        FileCheck::FileMissing => out.push(format!(
            "{file} not found ({place}). {why} Create it with [{section}] and these lines: {}. Then restart the game.",
            all_lines.join(", ")
        )),
        FileCheck::Unreadable(kind) => out.push(format!(
            "{file} could not be read ({kind}): {place}. Check that the file is not locked or protected."
        )),
        FileCheck::SettingsMissing(lines) => out.push(format!(
            "{file} ({place}) needs these lines under [{section}]: {}. {why} Restart the game afterwards.",
            lines.join(", ")
        )),
    }
}

#[cfg(test)]
#[path = "setup_merge_tests.rs"]
mod merge_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn temp_dir() -> PathBuf {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "tavern-ledger-setup-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    const POWER_OK: &str =
        "[Power]\nLogLevel=1\nFilePrinting=true\nConsolePrinting=false\nVerbose=true\n[Decks]\nLogLevel=1\n";

    fn power(text: &str) -> FileCheck {
        check_text(text, POWER_SECTION, &REQUIRED_POWER)
    }

    fn client(text: &str) -> FileCheck {
        check_text(text, LOG_SECTION, &REQUIRED_LOG)
    }

    #[test]
    fn power_log_enabled_is_ok() {
        assert_eq!(power(POWER_OK), FileCheck::Ok);
    }

    #[test]
    fn values_and_names_ignore_case_spaces_and_a_byte_order_mark() {
        let text = "\u{feff}; note\n[power]\n  LOGLEVEL = 1 \nfileprinting=TRUE\nVerbose=True\n";
        assert_eq!(power(text), FileCheck::Ok);
    }

    #[test]
    fn missing_section_lists_every_required_line() {
        assert_eq!(
            power("[Decks]\nLogLevel=1\n"),
            FileCheck::SettingsMissing(vec![
                "LogLevel=1".into(),
                "FilePrinting=true".into(),
                "Verbose=true".into()
            ])
        );
    }

    #[test]
    fn a_wrong_or_absent_value_is_listed() {
        let text = "[Power]\nLogLevel=1\nFilePrinting=false\n";
        assert_eq!(
            power(text),
            FileCheck::SettingsMissing(vec!["FilePrinting=true".into(), "Verbose=true".into()])
        );
    }

    #[test]
    fn keys_before_any_section_or_in_another_section_do_not_count() {
        let text = "LogLevel=1\n[Decks]\nFilePrinting=true\nVerbose=true\n";
        assert!(matches!(power(text), FileCheck::SettingsMissing(m) if m.len() == 3));
    }

    #[test]
    fn client_config_needs_an_unlimited_log_size() {
        assert_eq!(client("[Log]\nFileSizeLimit.Int=-1\n"), FileCheck::Ok);
        assert_eq!(
            client("[Log]\nFileSizeLimit.Int=10000\n"),
            FileCheck::SettingsMissing(vec!["FileSizeLimit.Int=-1".into()])
        );
        assert_eq!(
            client("[Graphics]\nWidth=1\n"),
            FileCheck::SettingsMissing(vec!["FileSizeLimit.Int=-1".into()])
        );
    }

    #[test]
    fn missing_file_missing_setting_and_unreadable_file_are_three_states() {
        let dir = temp_dir();
        let missing = dir.join("nope.config");
        let empty = dir.join("empty.config");
        fs::write(&empty, "").unwrap();
        // A folder where the file should be: it exists but cannot be read.
        let unreadable = dir.join("folder.config");
        fs::create_dir(&unreadable).unwrap();

        let report = check_setup(Some(&missing), Some(&empty));
        assert_eq!(report.log_config, FileCheck::FileMissing);
        assert!(matches!(
            report.client_config,
            FileCheck::SettingsMissing(_)
        ));
        let report = check_setup(Some(&unreadable), None);
        assert!(matches!(report.log_config, FileCheck::Unreadable(_)));
        // No path at all is "cannot check", not "missing".
        assert_eq!(report.client_config, FileCheck::CannotCheck);
    }

    #[test]
    fn a_good_setup_has_no_messages_and_a_bad_one_says_what_to_change() {
        let dir = temp_dir();
        let log = dir.join(LOG_CONFIG_FILE);
        let client_cfg = dir.join(CLIENT_CONFIG_FILE);
        fs::write(&log, POWER_OK).unwrap();
        fs::write(&client_cfg, "[Log]\nFileSizeLimit.Int=-1\n").unwrap();
        let good = check_setup(Some(&log), Some(&client_cfg));
        assert!(good.is_ok());
        assert!(good.messages().is_empty());

        fs::write(&client_cfg, "[Log]\n").unwrap();
        let bad = check_setup(Some(&log), Some(&client_cfg));
        let messages = bad.messages();
        assert_eq!(messages.len(), 1);
        assert!(messages[0].contains("FileSizeLimit.Int=-1"));
        assert!(messages[0].contains("[Log]"));
        assert!(messages[0].contains("Restart the game"));
    }

    #[test]
    fn a_missing_file_message_lists_the_whole_section_to_create() {
        let dir = temp_dir();
        let messages = check_setup(
            Some(&dir.join("log.config")),
            Some(&dir.join("client.config")),
        )
        .messages();
        assert_eq!(messages.len(), 2);
        assert!(messages[0].contains("LogLevel=1, FilePrinting=true, Verbose=true"));
        assert!(messages[1].contains("FileSizeLimit.Int=-1"));
    }

    #[test]
    fn the_check_never_writes_the_files() {
        let dir = temp_dir();
        let cfg = dir.join(CLIENT_CONFIG_FILE);
        fs::write(&cfg, "[Graphics]\nWidth=1\n").unwrap();
        let _ = check_setup(None, Some(&cfg)).messages();
        assert_eq!(fs::read_to_string(&cfg).unwrap(), "[Graphics]\nWidth=1\n");
        assert!(!dir.join(LOG_CONFIG_FILE).exists());
    }

    #[test]
    fn the_loading_screen_section_is_an_optional_note_not_a_problem() {
        let dir = temp_dir();
        let log = dir.join(LOG_CONFIG_FILE);
        let client_cfg = dir.join(CLIENT_CONFIG_FILE);
        fs::write(
            &client_cfg,
            "[Log]
FileSizeLimit.Int=-1
",
        )
        .unwrap();
        fs::write(&log, POWER_OK).unwrap();
        let without = check_setup(Some(&log), Some(&client_cfg));
        assert!(without.is_ok(), "the Power log is enough");
        assert!(without.messages().is_empty());
        let notes = without.notes();
        assert_eq!(notes.len(), 1);
        assert!(notes[0].starts_with("Optional:"));
        assert!(notes[0].contains("[LoadingScreen]"));
        assert!(notes[0].contains("LogLevel=1, FilePrinting=true"));

        let with = format!(
            "{POWER_OK}[LoadingScreen]
LogLevel=1
FilePrinting=True
Verbose=False
"
        );
        fs::write(&log, with).unwrap();
        assert!(check_setup(Some(&log), Some(&client_cfg))
            .notes()
            .is_empty());
    }

    #[test]
    fn no_optional_note_while_log_config_itself_is_missing() {
        let dir = temp_dir();
        assert!(check_setup(Some(&dir.join("log.config")), None)
            .notes()
            .is_empty());
    }

    #[test]
    fn no_game_folder_is_said_with_what_to_do() {
        use crate::discover::{FolderProblem, GameFolder, How, Resolution, Source};
        let found = Resolution::Found(GameFolder {
            dir: PathBuf::from("game"),
            how: How::Automatic(Source::Registry),
        });
        assert!(folder_messages(&found).is_empty());

        let none = folder_messages(&Resolution::NotFound);
        assert_eq!(none.len(), 1);
        assert!(none[0].contains("Choose the folder"));

        let gone = folder_messages(&Resolution::ChosenUnusable {
            dir: PathBuf::from(r"E:\Hearthstone"),
            problem: FolderProblem::Missing,
        });
        assert_eq!(gone.len(), 1);
        assert!(gone[0].contains(r"(E:\Hearthstone)"));
        assert!(gone[0].contains("That folder was not found."));
        assert!(gone[0].contains("go back to finding it automatically"));
    }

    fn install(root: &Path, name: &str, client: Option<&str>) -> PathBuf {
        let dir = root.join(name);
        fs::create_dir_all(dir.join("Logs")).unwrap();
        fs::write(dir.join("Hearthstone.exe"), b"").unwrap();
        if let Some(text) = client {
            fs::write(dir.join(CLIENT_CONFIG_FILE), text).unwrap();
        }
        dir
    }

    fn local_with_good_log(root: &Path) -> PathBuf {
        let local = root.join("local");
        let log = log_config_target(&local);
        fs::create_dir_all(log.parent().unwrap()).unwrap();
        fs::write(&log, POWER_OK).unwrap();
        local
    }

    #[test]
    fn the_check_follows_the_install_folder_in_use() {
        let root = temp_dir();
        let local = local_with_good_log(&root);
        let automatic = install(&root, "E", None);
        let chosen = install(
            &root,
            "D",
            Some(
                "[Log]
FileSizeLimit.Int=-1
",
            ),
        );

        let before = check_install(Some(&local), Some(&automatic));
        assert!(!before.is_ok());
        assert_eq!(before.client_config, FileCheck::FileMissing);

        // The player chooses the other install: the same check, its folder.
        let after = check_install(Some(&local), Some(&chosen));
        assert!(after.is_ok());
        assert!(after.messages().is_empty());

        // And back to automatic: the problem is there again, with its path.
        let back = check_install(Some(&local), Some(&automatic));
        assert_eq!(back.messages().len(), 1);
        assert!(back.messages()[0].contains(&automatic.display().to_string()));
    }

    #[test]
    fn a_player_who_fixes_the_file_by_hand_clears_the_next_check() {
        let root = temp_dir();
        let local = local_with_good_log(&root);
        let game = install(
            &root,
            "D",
            Some(
                "[Log]
FileSizeLimit.Int=10000
",
            ),
        );
        assert!(!check_install(Some(&local), Some(&game)).is_ok());
        fs::write(
            game.join(CLIENT_CONFIG_FILE),
            "[Log]
FileSizeLimit.Int=-1
",
        )
        .unwrap();
        assert!(check_install(Some(&local), Some(&game)).is_ok());
    }

    #[test]
    fn a_folder_the_app_does_not_know_is_cannot_check_never_ok_or_missing() {
        let root = temp_dir();
        let local = local_with_good_log(&root);
        let no_folder = check_install(Some(&local), None);
        assert_eq!(no_folder.client_config, FileCheck::CannotCheck);
        assert_eq!(no_folder.log_config, FileCheck::Ok);
        assert!(!no_folder.is_ok());
        let said = no_folder.messages();
        assert_eq!(said.len(), 1);
        assert!(said[0].starts_with("client.config could not be checked"));

        let no_local = check_install(
            None,
            Some(&install(
                &root,
                "D",
                Some(
                    "[Log]
FileSizeLimit.Int=-1
",
                ),
            )),
        );
        assert_eq!(no_local.log_config, FileCheck::CannotCheck);
        assert!(!no_local.is_ok());
        assert!(no_local.messages()[0].starts_with("log.config could not be checked"));
        // The optional note is never offered when log.config was not read.
        assert!(no_local.notes().is_empty());
    }

    #[test]
    fn client_config_is_next_to_the_game_not_in_logs() {
        let game = Path::new("game").join("Hearthstone");
        assert_eq!(
            client_config_path(&game.join("Logs")),
            Some(game.join(CLIENT_CONFIG_FILE))
        );
    }
}
