//! Is the game set up to write a complete `Power.log`? Two files decide it
//! (T-106): `log.config` must enable the Power log, and `client.config` next
//! to `Hearthstone.exe` must lift the log size limit (without that the game
//! stops writing at about 10 MB). Read-only: this module never writes either
//! file, it only says what to change (D-004).

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const LOG_CONFIG_FILE: &str = "log.config";
pub const CLIENT_CONFIG_FILE: &str = "client.config";

const POWER_SECTION: &str = "Power";
const REQUIRED_POWER: [(&str, &str); 3] = [
    ("LogLevel", "1"),
    ("FilePrinting", "true"),
    ("Verbose", "true"),
];
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
    fn client_config_is_next_to_the_game_not_in_logs() {
        assert_eq!(
            client_config_path(Path::new(r"C:\Game\Hearthstone\Logs")),
            Some(Path::new(r"C:\Game\Hearthstone").join(CLIENT_CONFIG_FILE))
        );
    }
}
