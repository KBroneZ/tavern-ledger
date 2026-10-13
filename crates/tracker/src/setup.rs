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
        return FileCheck::FileMissing;
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
        assert_eq!(report.client_config, FileCheck::FileMissing);
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
        let messages = check_setup(Some(&dir.join("log.config")), None).messages();
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

    #[test]
    fn client_config_is_next_to_the_game_not_in_logs() {
        let game = Path::new("game").join("Hearthstone");
        assert_eq!(
            client_config_path(&game.join("Logs")),
            Some(game.join(CLIENT_CONFIG_FILE))
        );
    }
}
