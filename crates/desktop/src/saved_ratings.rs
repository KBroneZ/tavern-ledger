//! Keeps the own rating read from the game (D-063) with each game on this PC
//! (T-366, D-147), for the lobby panel's session recap. The file and its rules
//! are `tracker::game_ratings`; this is when the app uses them.
//!
//! Only the window that owns the history saves them, once the read has a
//! rating two reads agreed on. They never go into a report, so they are
//! neither uploaded nor in a problem report; turning the switch off deletes
//! them all, and with it off nothing is saved.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tracker::game_ratings::{SavedRatings, Taken, FILE_NAME};

use crate::game_memory::TakenReading;
use crate::overlay::lock;

#[derive(Debug, Default)]
pub struct Kept {
    saved: Mutex<Option<SavedRatings>>,
    /// The data folder, known to every window, so turning the switch off
    /// deletes the file even where it is not open.
    dir: Mutex<Option<PathBuf>>,
}

/// What a read did to the saved ratings.
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
        let opened = SavedRatings::open(data_dir).map_err(|e| {
            format!(
                "Cannot open the ratings saved from the game ({:?}); the lobby panel shows no rating changes.",
                e.kind()
            )
        })?;
        self.set_dir(data_dir);
        *lock(&self.saved) = Some(opened);
        Ok(())
    }

    /// Runs `f` with the saved ratings (none when not open).
    pub fn with<R>(&self, f: impl FnOnce(Option<&SavedRatings>) -> R) -> R {
        f(lock(&self.saved).as_ref())
    }

    /// After a read: saves the rating taken while a game was the newest one
    /// seen (or before any game), with its freshness. `still_on` is asked
    /// under the lock, just before a save, so a switch turned off since the
    /// read (which deletes the file under the same lock) is never undone.
    pub fn after_read(&self, taken: Option<TakenReading>, still_on: impl Fn() -> bool) -> Outcome {
        let Some(read) = taken else {
            return Outcome::Unchanged;
        };
        let mut saved = lock(&self.saved);
        let Some(saved) = saved.as_mut() else {
            return Outcome::Unchanged;
        };
        if !still_on() {
            return Outcome::Unchanged;
        }
        let taken = Taken {
            after: read.after.as_ref(),
            previous: read.previous.as_ref(),
            game_duos: read.game_duos,
            first_of_run: read.first_of_run,
            solo: read.solo,
            duos: read.duos,
        };
        match saved.record(taken) {
            Ok(true) => Outcome::Changed,
            Ok(false) => Outcome::Unchanged,
            Err(e) => Outcome::Failed(format!(
                "The rating read from the game could not be saved ({:?}).",
                e.kind()
            )),
        }
    }

    /// The switch was turned off: every saved rating goes, the file too when
    /// this window does not have it open.
    pub fn forget_all(&self) -> Result<(), String> {
        let failed = |e: io::Error| {
            format!(
                "The ratings saved from the game could not be deleted ({:?}).",
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
    use tracker::store::GameKey;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tl-saved-ratings-{tag}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn key() -> GameKey {
        GameKey {
            session: "Hearthstone_2026_10_14_20_00_00".into(),
            index: 1,
        }
    }

    fn taken(solo: u32) -> Option<TakenReading> {
        Some(TakenReading {
            after: Some(key()),
            previous: None,
            game_duos: Some(false),
            first_of_run: true,
            solo: Some(solo),
            duos: None,
        })
    }

    #[test]
    fn a_rating_taken_is_saved_once_and_replaced_by_a_later_one() {
        let dir = temp_dir("keep");
        let kept = Kept::default();
        kept.open(&dir).unwrap();
        assert_eq!(kept.after_read(taken(6400), || true), Outcome::Changed);
        assert_eq!(kept.after_read(taken(6400), || true), Outcome::Unchanged);
        assert_eq!(kept.after_read(taken(6425), || true), Outcome::Changed);
        kept.with(|r| {
            assert_eq!(r.unwrap().after(Some(&key())).unwrap().solo, Some(6425));
        });
    }

    #[test]
    fn nothing_is_saved_without_a_rating_or_an_open_file_or_a_switch_still_on() {
        let kept = Kept::default();
        assert_eq!(
            kept.after_read(taken(6400), || true),
            Outcome::Unchanged,
            "not the window that owns the history"
        );
        let dir = temp_dir("none");
        kept.open(&dir).unwrap();
        assert_eq!(kept.after_read(None, || true), Outcome::Unchanged);
        assert_eq!(kept.after_read(taken(6400), || false), Outcome::Unchanged);
        assert!(!dir.join(FILE_NAME).exists());
    }

    #[test]
    fn an_impossible_rating_is_refused_with_words() {
        let kept = Kept::default();
        kept.open(&temp_dir("range")).unwrap();
        assert!(matches!(
            kept.after_read(taken(0), || true),
            Outcome::Failed(_)
        ));
    }

    #[test]
    fn turning_the_switch_off_deletes_the_file_also_where_it_is_not_open() {
        let dir = temp_dir("forget");
        let kept = Kept::default();
        kept.open(&dir).unwrap();
        kept.after_read(taken(6400), || true);
        assert!(dir.join(FILE_NAME).exists());
        kept.forget_all().unwrap();
        assert!(!dir.join(FILE_NAME).exists());
        // The read-only window, or a file that could not be read.
        std::fs::write(dir.join(FILE_NAME), "not json").unwrap();
        let other = Kept::default();
        assert!(other.open(&dir).is_err());
        other.set_dir(&dir);
        other.forget_all().unwrap();
        assert!(!dir.join(FILE_NAME).exists());
    }
}
