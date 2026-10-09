//! Where the game writes its logs: `<install>\Logs\Hearthstone_<date>\`.

use std::path::{Path, PathBuf};
use std::process::Command;

const UNINSTALL_KEYS: [&str; 2] = [
    r"HKLM\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\Hearthstone",
    r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\Hearthstone",
];
const FALLBACK_INSTALL_DIRS: [&str; 2] = [
    r"C:\Program Files (x86)\Hearthstone",
    r"C:\Program Files\Hearthstone",
];

/// The game's Logs folder, from the install path in the registry or the
/// default install folders. Read-only: runs `reg query`, writes nothing.
pub fn find_logs_dir() -> Option<PathBuf> {
    let from_registry = UNINSTALL_KEYS
        .iter()
        .filter_map(|key| install_location(key));
    let fallbacks = FALLBACK_INSTALL_DIRS.iter().map(PathBuf::from);
    from_registry
        .chain(fallbacks)
        .map(|dir| dir.join("Logs"))
        .find(|logs| logs.is_dir())
}

fn install_location(key: &str) -> Option<PathBuf> {
    // Full path: never a reg.exe found next to the app or on PATH.
    let system_root = std::env::var_os("SystemRoot").unwrap_or_else(|| r"C:\Windows".into());
    let reg = PathBuf::from(system_root).join("System32").join("reg.exe");
    let output = Command::new(reg)
        .args(["query", key, "/v", "InstallLocation"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    parse_reg_value(&String::from_utf8_lossy(&output.stdout), "InstallLocation")
}

/// Parses `    InstallLocation    REG_SZ    C:\Path` from `reg query` output.
pub fn parse_reg_value(output: &str, name: &str) -> Option<PathBuf> {
    output.lines().find_map(|line| {
        let rest = line.trim_start().strip_prefix(name)?;
        let (_, value) = rest.trim_start().split_once("REG_SZ")?;
        let value = value.trim();
        (!value.is_empty()).then(|| PathBuf::from(value))
    })
}

/// True for folder names like `Hearthstone_2026_10_09_15_26_25`.
pub fn is_session_name(name: &str) -> bool {
    let Some(stamp) = name.strip_prefix("Hearthstone_") else {
        return false;
    };
    let parts: Vec<&str> = stamp.split('_').collect();
    let widths = [4, 2, 2, 2, 2, 2];
    parts.len() == widths.len()
        && parts
            .iter()
            .zip(widths)
            .all(|(p, w)| p.len() == w && p.bytes().all(|b| b.is_ascii_digit()))
}

/// The newest session folder (names sort by start time).
pub fn latest_session(logs_dir: &Path) -> Option<PathBuf> {
    std::fs::read_dir(logs_dir)
        .ok()?
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .filter(|e| is_session_name(&e.file_name().to_string_lossy()))
        .map(|e| e.path())
        .max()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_install_location_from_reg_output() {
        let out = "\r\nHKEY_LOCAL_MACHINE\\SOFTWARE\\X\r\n    InstallLocation    REG_SZ    C:\\Battle.net\\Hearthstone\r\n\r\n";
        assert_eq!(
            parse_reg_value(out, "InstallLocation"),
            Some(PathBuf::from(r"C:\Battle.net\Hearthstone"))
        );
        assert_eq!(parse_reg_value("nothing here", "InstallLocation"), None);
    }

    #[test]
    fn recognizes_session_folders() {
        assert!(is_session_name("Hearthstone_2026_10_09_15_26_25"));
        assert!(!is_session_name("Hearthstone_2026_10_09"));
        assert!(!is_session_name("Hearthstone_2026_10_09_15_26_2x"));
        assert!(!is_session_name("Other_2026_10_09_15_26_25"));
    }
}
