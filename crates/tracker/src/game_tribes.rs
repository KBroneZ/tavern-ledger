//! The lobby's five tribes read from the game (D-063), saved with each game
//! on this PC (T-540, D-104, D-108), so the recap and the stats can show the
//! exact tribes after the game instead of the ones seen in the tavern.
//!
//! They live in their own file next to the history (`game_tribes.json`),
//! never inside a game's report: the report is what is uploaded and what a
//! problem report holds, and neither may carry them. Re-reading a game
//! (T-107) rewrites its report and keeps them. Only tribe ids from
//! [`crate::lobby_tribes::TRIBES`] are kept, five per game.
//!
//! A game's tribes are saved only once they passed every check the overlay
//! uses (a build the reader was checked on, five known tribes, the tavern
//! confirming none outside them, see [`crate::game_values`]), and dropped
//! when the tavern contradicts them later in the game. Turning the switch
//! off deletes the file.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::game_values::{GameTribes, ReaderStatus};
use crate::lobby_tribes::validate;
use crate::possible::{confirmed_tribes, PoolCard};
use crate::store::GameKey;

pub const FILE_NAME: &str = "game_tribes.json";
const FORMAT_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
struct FileFormat {
    version: u32,
    games: Vec<FileGame>,
}

#[derive(Serialize, Deserialize)]
struct FileGame {
    session: String,
    index: u64,
    tribes: Vec<String>,
}

#[derive(Debug)]
pub struct SavedTribes {
    path: PathBuf,
    games: BTreeMap<GameKey, Vec<String>>,
}

impl SavedTribes {
    /// Loads the file; a missing file is an empty set. A file that cannot be
    /// read is an error (and is left alone), never silently replaced.
    pub fn open(dir: &Path) -> io::Result<SavedTribes> {
        fs::create_dir_all(dir)?;
        let path = dir.join(FILE_NAME);
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                return Ok(SavedTribes {
                    path,
                    games: BTreeMap::new(),
                })
            }
            Err(e) => return Err(e),
        };
        let invalid = |what: &str| io::Error::new(io::ErrorKind::InvalidData, what.to_string());
        let file: FileFormat =
            serde_json::from_str(&text).map_err(|_| invalid("unreadable game tribes file"))?;
        if file.version != FORMAT_VERSION {
            return Err(invalid("unknown game tribes file version"));
        }
        let mut games = BTreeMap::new();
        for game in file.games {
            let tribes = validate(&game.tribes).map_err(|_| invalid("bad game tribes entry"))?;
            let key = GameKey {
                session: game.session,
                index: game.index,
            };
            games.insert(key, tribes);
        }
        Ok(SavedTribes { path, games })
    }

    pub fn get(&self, key: &GameKey) -> Option<&[String]> {
        self.games.get(key).map(Vec::as_slice)
    }

    pub fn len(&self) -> usize {
        self.games.len()
    }

    pub fn is_empty(&self) -> bool {
        self.games.is_empty()
    }

    /// Saves a game's tribes. True when the file changed; the same tribes
    /// again write nothing.
    pub fn record(&mut self, key: &GameKey, tribes: &[String]) -> io::Result<bool> {
        let tribes = validate(tribes)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?;
        if self.games.get(key) == Some(&tribes) {
            return Ok(false);
        }
        let mut games = self.games.clone();
        games.insert(key.clone(), tribes);
        self.commit(games).map(|()| true)
    }

    /// Drops a game's tribes. True when it had some.
    pub fn remove(&mut self, key: &GameKey) -> io::Result<bool> {
        if !self.games.contains_key(key) {
            return Ok(false);
        }
        let mut games = self.games.clone();
        games.remove(key);
        self.commit(games).map(|()| true)
    }

    /// Deletes every saved game's tribes and the file (the switch turned off).
    pub fn forget_all(&mut self) -> io::Result<()> {
        match fs::remove_file(&self.path) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        self.games.clear();
        Ok(())
    }

    /// Writes the new state to disk and only then takes it, so a failed
    /// write changes nothing. The file is replaced whole (temp file, rename).
    fn commit(&mut self, games: BTreeMap<GameKey, Vec<String>>) -> io::Result<()> {
        let file = FileFormat {
            version: FORMAT_VERSION,
            games: games
                .iter()
                .map(|(key, tribes)| FileGame {
                    session: key.session.clone(),
                    index: key.index,
                    tribes: tribes.clone(),
                })
                .collect(),
        };
        let text = serde_json::to_string_pretty(&file).map_err(io::Error::other)?;
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, text)?;
        fs::rename(&tmp, &self.path)?;
        self.games = games;
        Ok(())
    }
}

/// What to do with the game in progress's saved tribes after a read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Keep {
    /// The read passed every check: save these five.
    Save(Vec<String>),
    /// The tavern contradicted the read: drop what was saved.
    Drop,
    /// Nothing usable yet (reader off, waiting, unsupported): change nothing.
    Nothing,
}

/// Checks the reader's tribes for the game in progress the way the overlay
/// does: `build` is the game's build from the log, `offered` the minions the
/// tavern offered so far, `pool` the card data's minion pool.
pub fn keep(
    tribes: Option<&[String]>,
    status: ReaderStatus,
    build: Option<i64>,
    offered: &[String],
    pool: &[PoolCard],
) -> Keep {
    // The reader refused this game's read (the overlay found the tavern
    // contradicting it): whatever was saved for it goes.
    if status == ReaderStatus::UnsupportedVersion {
        return Keep::Drop;
    }
    let read = GameTribes::new(tribes, status, build);
    if read.tribes.is_empty() {
        return Keep::Nothing;
    }
    let confirmed = confirmed_tribes(offered, pool);
    let checked = read.checked_against(confirmed.as_deref());
    if checked.tribes.is_empty() {
        Keep::Drop
    } else {
        Keep::Save(checked.tribes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_values::CHECKED_BUILDS;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tl-game-tribes-{tag}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn key(index: u64) -> GameKey {
        GameKey {
            session: "Hearthstone_2026_10_11_02_56_10".into(),
            index,
        }
    }

    fn strings(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    fn lobby() -> Vec<String> {
        strings(&["UNDEAD", "BEAST", "DEMON", "MURLOC", "NAGA"])
    }

    #[test]
    fn tribes_are_saved_per_game_sorted_and_read_back() {
        let dir = temp_dir("save");
        let mut saved = SavedTribes::open(&dir).unwrap();
        assert!(saved.is_empty(), "no file: nothing saved, not an error");
        assert!(saved.record(&key(1), &lobby()).unwrap());
        assert!(
            !saved.record(&key(1), &lobby()).unwrap(),
            "the same again writes nothing"
        );
        let again = SavedTribes::open(&dir).unwrap();
        assert_eq!(
            again.get(&key(1)).unwrap(),
            strings(&["BEAST", "DEMON", "MURLOC", "NAGA", "UNDEAD"])
        );
        assert_eq!(again.get(&key(2)), None);
    }

    #[test]
    fn only_five_known_tribes_are_kept() {
        let dir = temp_dir("refuse");
        let mut saved = SavedTribes::open(&dir).unwrap();
        for bad in [
            strings(&["BEAST", "DEMON"]),
            strings(&["BEAST", "DEMON", "MURLOC", "NAGA", "Some Player#1234"]),
            strings(&["BEAST", "BEAST", "MURLOC", "NAGA", "UNDEAD"]),
        ] {
            assert!(saved.record(&key(1), &bad).is_err());
        }
        assert!(saved.is_empty());
        assert!(!dir.join(FILE_NAME).exists(), "nothing written");
    }

    #[test]
    fn a_broken_or_edited_file_is_an_error_and_left_alone() {
        let dir = temp_dir("broken");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(FILE_NAME), "not json").unwrap();
        assert!(SavedTribes::open(&dir).is_err());
        let edited = r#"{"version":1,"games":[{"session":"s","index":1,"tribes":["BEAST","x","y","z","w"]}]}"#;
        fs::write(dir.join(FILE_NAME), edited).unwrap();
        assert!(SavedTribes::open(&dir).is_err());
        assert_eq!(fs::read_to_string(dir.join(FILE_NAME)).unwrap(), edited);
        fs::write(dir.join(FILE_NAME), r#"{"version":9,"games":[]}"#).unwrap();
        assert!(SavedTribes::open(&dir).is_err());
    }

    #[test]
    fn a_game_can_be_dropped_and_everything_forgotten() {
        let dir = temp_dir("forget");
        let mut saved = SavedTribes::open(&dir).unwrap();
        saved.record(&key(1), &lobby()).unwrap();
        saved.record(&key(2), &lobby()).unwrap();
        assert!(saved.remove(&key(1)).unwrap());
        assert!(!saved.remove(&key(1)).unwrap());
        assert_eq!(SavedTribes::open(&dir).unwrap().len(), 1);
        saved.forget_all().unwrap();
        assert!(saved.is_empty());
        assert!(!dir.join(FILE_NAME).exists());
        saved.forget_all().unwrap();
    }

    fn pool() -> Vec<PoolCard> {
        let card = |id: &str, tribe: &str| PoolCard {
            id: id.into(),
            tier: 1,
            tribes: vec![tribe.into()],
            duos_only: false,
        };
        vec![card("BEAST_1", "BEAST"), card("MECH_1", "MECHANICAL")]
    }

    #[test]
    fn a_read_is_kept_only_when_it_passes_the_overlays_checks() {
        let build = Some(CHECKED_BUILDS[0]);
        let ok = ReaderStatus::Ok;
        let offered = strings(&["BEAST_1"]);
        assert_eq!(
            keep(Some(&lobby()), ok, build, &offered, &pool()),
            Keep::Save(strings(&["BEAST", "DEMON", "MURLOC", "NAGA", "UNDEAD"]))
        );
        // The tavern offered a mech, which is not among the five: wrong read.
        let mech = strings(&["BEAST_1", "MECH_1"]);
        assert_eq!(keep(Some(&lobby()), ok, build, &mech, &pool()), Keep::Drop);
        // A build the reader was not checked on, the reader waiting, no read.
        assert_eq!(
            keep(Some(&lobby()), ok, Some(1), &offered, &pool()),
            Keep::Nothing
        );
        assert_eq!(
            keep(Some(&lobby()), ok, None, &offered, &pool()),
            Keep::Nothing
        );
        assert_eq!(
            keep(None, ReaderStatus::Waiting, build, &offered, &pool()),
            Keep::Nothing
        );
        assert_eq!(
            keep(None, ReaderStatus::Off, build, &[], &[]),
            Keep::Nothing
        );
        // The reader refused the read for this game: drop what was saved.
        assert_eq!(
            keep(None, ReaderStatus::UnsupportedVersion, build, &[], &[]),
            Keep::Drop
        );
    }
}
