//! Keeps the lobby tribes read from the game (D-063) with each game on this
//! PC (T-540, D-104), for the recap and the stats. The checks and the file
//! are `tracker::game_tribes`; this is when the app uses them.
//!
//! Only the window that owns the history saves them (the one the tracker
//! runs in). A read is saved once it passes the overlay's checks, during the
//! game, and dropped if the tavern contradicts it later; turning the switch
//! off deletes them all. They never go into a report, so they are neither
//! uploaded nor in a problem report.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tracker::game_tribes::{keep, Keep, SavedTribes, FILE_NAME};
use tracker::game_values::ReaderStatus;
use tracker::live::LiveGame;
use tracker::possible::PoolCard;
use tracker::store::GameKey;

use crate::overlay::lock;

#[derive(Debug, Default)]
pub struct Kept {
    saved: Mutex<Option<SavedTribes>>,
    /// The data folder, known to every window, so turning the switch off
    /// deletes the file even where it is not open (the read-only window, or
    /// a file that could not be read).
    dir: Mutex<Option<PathBuf>>,
}

/// What a read of the game in progress did to the saved tribes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Unchanged,
    Changed,
    /// The file could not be written: the message for the status line.
    Failed(String),
}

impl Kept {
    /// Where the file lives; every window calls this at start.
    pub fn set_dir(&self, data_dir: &Path) {
        *lock(&self.dir) = Some(data_dir.to_path_buf());
    }

    /// Opens the file next to the history. Only the window that owns the
    /// history calls this; the other one shows none.
    pub fn open(&self, data_dir: &Path) -> Result<(), String> {
        let opened = SavedTribes::open(data_dir).map_err(|e| {
            format!(
                "Cannot open the tribes saved from the game ({:?}); the recap shows the tavern's.",
                e.kind()
            )
        })?;
        self.set_dir(data_dir);
        *lock(&self.saved) = Some(opened);
        Ok(())
    }

    /// The tribes saved for one game, if any.
    pub fn get(&self, key: &GameKey) -> Option<Vec<String>> {
        lock(&self.saved).as_ref()?.get(key).map(<[String]>::to_vec)
    }

    /// Runs `f` with every game's saved tribes (none when not open).
    pub fn with<R>(&self, f: impl FnOnce(Option<&SavedTribes>) -> R) -> R {
        f(lock(&self.saved).as_ref())
    }

    /// After a read: saves the game in progress's tribes once they pass the
    /// checks, or drops them when the tavern contradicted them. `still_on` is
    /// asked under the lock, just before a save, so a switch turned off since
    /// the read (which deletes the file under the same lock) is never undone.
    pub fn after_read(
        &self,
        game: Option<&GameKey>,
        read: (Option<Vec<String>>, ReaderStatus),
        live: Option<&LiveGame>,
        pool: &[PoolCard],
        still_on: impl Fn() -> bool,
    ) -> Outcome {
        let (Some(game), Some(live)) = (game, live) else {
            return Outcome::Unchanged;
        };
        let mut saved = lock(&self.saved);
        let Some(saved) = saved.as_mut() else {
            return Outcome::Unchanged;
        };
        let decided = keep(read.0.as_deref(), read.1, live.build, &live.offered, pool);
        let done = match decided {
            Keep::Save(_) if !still_on() => Ok(false),
            Keep::Save(tribes) => saved.record(game, &tribes),
            Keep::Drop => saved.remove(game),
            Keep::Nothing => Ok(false),
        };
        match done {
            Ok(true) => Outcome::Changed,
            Ok(false) => Outcome::Unchanged,
            Err(e) => Outcome::Failed(format!(
                "The tribes read from the game could not be saved ({:?}).",
                e.kind()
            )),
        }
    }

    /// The switch was turned off: every saved game's tribes go, the file too
    /// when this window does not have it open.
    pub fn forget_all(&self) -> Result<(), String> {
        let failed = |e: io::Error| {
            format!(
                "The tribes saved from the game could not be deleted ({:?}).",
                e.kind()
            )
        };
        let mut saved = lock(&self.saved);
        if let Some(saved) = saved.as_mut() {
            return saved.forget_all().map_err(failed);
        }
        let Some(dir) = lock(&self.dir).clone() else {
            return Ok(());
        };
        match std::fs::remove_file(dir.join(FILE_NAME)) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(failed(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tracker::game_values::CHECKED_BUILDS;
    use tracker::live::live_game;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tl-saved-tribes-{tag}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn key() -> GameKey {
        GameKey {
            session: "Hearthstone_2026_10_11_02_56_10".into(),
            index: 1,
        }
    }

    fn lobby() -> Vec<String> {
        ["BEAST", "DEMON", "MURLOC", "NAGA", "UNDEAD"]
            .iter()
            .map(|t| t.to_string())
            .collect()
    }

    /// A game in progress on a checked build, the tavern having offered `offered`.
    fn live(offered: &[&str]) -> LiveGame {
        let mut game = live_game(&serde_json::json!({
            "status": "incomplete", "game_type": "GT_BATTLEGROUNDS", "local_player_id": 1,
            "hero": "H", "build": CHECKED_BUILDS[0],
            "lobby": [{"player_id": 1, "hero": "H"}], "rounds": [],
        }));
        game.offered = offered.iter().map(|s| s.to_string()).collect();
        game
    }

    fn pool() -> Vec<PoolCard> {
        vec![PoolCard {
            id: "MECH_1".into(),
            tier: 1,
            tribes: vec!["MECHANICAL".into()],
            duos_only: false,
        }]
    }

    #[test]
    fn a_checked_read_is_saved_and_a_contradicted_one_dropped() {
        let dir = temp_dir("keep");
        let kept = Kept::default();
        kept.open(&dir).unwrap();
        let read = (Some(lobby()), ReaderStatus::Ok);
        let game = live(&[]);
        assert_eq!(
            kept.after_read(Some(&key()), read.clone(), Some(&game), &pool(), || true),
            Outcome::Changed
        );
        assert_eq!(kept.get(&key()), Some(lobby()));
        assert_eq!(
            kept.after_read(Some(&key()), read.clone(), Some(&game), &pool(), || true),
            Outcome::Unchanged
        );
        // Later the tavern offers a mech: the read was wrong, so it goes.
        let mech = live(&["MECH_1"]);
        assert_eq!(
            kept.after_read(Some(&key()), read, Some(&mech), &pool(), || true),
            Outcome::Changed
        );
        assert_eq!(kept.get(&key()), None);
    }

    #[test]
    fn nothing_is_saved_without_a_game_a_read_or_an_open_file() {
        let kept = Kept::default();
        let read = (Some(lobby()), ReaderStatus::Ok);
        let game = live(&[]);
        assert_eq!(
            kept.after_read(Some(&key()), read.clone(), Some(&game), &pool(), || true),
            Outcome::Unchanged,
            "not the window that owns the history"
        );
        kept.open(&temp_dir("none")).unwrap();
        assert_eq!(
            kept.after_read(None, read.clone(), Some(&game), &pool(), || true),
            Outcome::Unchanged
        );
        assert_eq!(
            kept.after_read(Some(&key()), read, None, &pool(), || true),
            Outcome::Unchanged
        );
        let off = (None, ReaderStatus::Off);
        assert_eq!(
            kept.after_read(Some(&key()), off, Some(&game), &pool(), || true),
            Outcome::Unchanged
        );
        assert_eq!(kept.get(&key()), None);
    }

    #[test]
    fn turning_the_switch_off_deletes_them_all() {
        let dir = temp_dir("forget");
        let kept = Kept::default();
        kept.open(&dir).unwrap();
        let game = live(&[]);
        kept.after_read(
            Some(&key()),
            (Some(lobby()), ReaderStatus::Ok),
            Some(&game),
            &pool(),
            || true,
        );
        assert!(dir.join(tracker::game_tribes::FILE_NAME).exists());
        kept.forget_all().unwrap();
        assert_eq!(kept.get(&key()), None);
        assert!(!dir.join(tracker::game_tribes::FILE_NAME).exists());
    }

    #[test]
    fn a_read_taken_before_the_switch_went_off_is_not_saved_after() {
        let kept = Kept::default();
        kept.open(&temp_dir("race")).unwrap();
        let read = (Some(lobby()), ReaderStatus::Ok);
        let game = live(&[]);
        let outcome = kept.after_read(Some(&key()), read, Some(&game), &pool(), || false);
        assert_eq!(outcome, Outcome::Unchanged);
        assert_eq!(kept.get(&key()), None);
    }

    #[test]
    fn off_deletes_the_file_also_where_it_is_not_open() {
        // The read-only window, or a file that could not be read.
        let dir = temp_dir("closed");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(FILE_NAME), "not json").unwrap();
        let kept = Kept::default();
        assert!(kept.open(&dir).is_err());
        kept.set_dir(&dir);
        kept.forget_all().unwrap();
        assert!(!dir.join(FILE_NAME).exists());
        kept.forget_all().unwrap();
    }
}
