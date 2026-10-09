//! `upload.json` next to the history: whether the user turned upload on and
//! which account is signed in (id and email, no tokens). Upload is off unless
//! this file says otherwise, so a new install, a missing or broken file, or an
//! update never turns it on.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const FILE_NAME: &str = "upload.json";

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
}

pub fn path(dir: &Path) -> PathBuf {
    dir.join(FILE_NAME)
}

/// The saved settings; anything unreadable is "off, signed out".
pub fn load(dir: &Path) -> Settings {
    fs::read(path(dir))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

/// Writes a temporary file and renames it over the old one, so a crash
/// leaves either the old settings or the new ones.
pub fn save(dir: &Path, settings: &Settings) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    let tmp = dir.join(format!("{FILE_NAME}.tmp"));
    fs::write(
        &tmp,
        serde_json::to_vec_pretty(settings).map_err(io::Error::other)?,
    )?;
    fs::rename(&tmp, path(dir))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::queue::tests::temp_dir;

    #[test]
    fn off_unless_the_user_turned_it_on() {
        let dir = temp_dir("settings");
        assert!(!load(&dir).enabled, "no file: off");
        fs::write(path(&dir), "{broken").unwrap();
        assert!(!load(&dir).enabled, "broken file: off");
        fs::write(path(&dir), "{\"future_field\": 1}").unwrap();
        assert!(
            !load(&dir).enabled,
            "a file from another version without the switch: off"
        );
        let on = Settings {
            enabled: true,
            user_id: Some("u".into()),
            email: None,
        };
        save(&dir, &on).unwrap();
        assert_eq!(load(&dir), on);
        assert!(!dir.join(format!("{FILE_NAME}.tmp")).exists());
    }
}
