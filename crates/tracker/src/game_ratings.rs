//! The player's own rating, read from the game (D-063) after each game and
//! kept with it on this PC only (T-366, D-147), so the lobby panel can show the
//! rating won or lost per game, the rating a session started with and the one
//! it has now.
//!
//! One reading is kept for each game the app has seen, the games played with
//! the reconnect dev tool included (they move the rating too): the latest
//! rating two reads agreed on while that game was the newest one, so it is the
//! rating after that game; and one for the time before any game. They live in
//! their own file next to the history (`game_ratings.json`), never inside a
//! game's report: the report is what is uploaded and what a problem report
//! holds, and neither may carry them. Turning the read off deletes the file;
//! with the read off nothing is saved.
//!
//! The game may keep showing the old rating for a moment after a game, so each
//! reading says, per mode, whether its value is *fresh* (known to include the
//! game it follows). It is fresh when it is the first rating the app took, or
//! when the reading right before it (the game before, in the history) was fresh
//! and the value moved in the mode of the game just played, or stayed the same
//! in the other mode. An unchanged value in the mode just played may be stale,
//! so it is not fresh.
//!
//! The change a game made follows D-069's rule and is never a guess: its
//! rating minus the one after the game right before it, in the game's mode,
//! only when both readings exist, the earlier one is fresh and the value moved.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::store::GameKey;

pub const FILE_NAME: &str = "game_ratings.json";
const FORMAT_VERSION: u32 = 1;
/// The game's own range for a Battlegrounds rating (D-069).
const MIN_RATING: u32 = 1;
const MAX_RATING: u32 = 30_000;

/// The two ratings the game showed at one read, and whether each is fresh.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reading {
    pub solo: Option<u32>,
    pub duos: Option<u32>,
    pub fresh_solo: bool,
    pub fresh_duos: bool,
}

impl Reading {
    /// The rating of one mode.
    pub fn of(&self, duos: bool) -> Option<u32> {
        if duos {
            self.duos
        } else {
            self.solo
        }
    }

    /// Whether the rating of one mode is known to include the game it follows.
    pub fn fresh(&self, duos: bool) -> bool {
        if duos {
            self.fresh_duos
        } else {
            self.fresh_solo
        }
    }

    fn in_range(&self) -> bool {
        [self.solo, self.duos]
            .into_iter()
            .flatten()
            .all(|r| (MIN_RATING..=MAX_RATING).contains(&r))
    }
}

/// What the app knows when it saves a reading.
#[derive(Clone, Copy, Debug)]
pub struct Taken<'a> {
    /// The newest game the app has seen (`None`: before any game).
    pub after: Option<&'a GameKey>,
    /// The game right before it.
    pub previous: Option<&'a GameKey>,
    /// Whether the game `after` was Duos (`None`: unknown or another mode).
    pub game_duos: Option<bool>,
    /// The first rating the app took since it started reading.
    pub first_of_run: bool,
    pub solo: Option<u32>,
    pub duos: Option<u32>,
}

#[derive(Serialize, Deserialize)]
struct FileFormat {
    version: u32,
    /// Before any game of the history.
    initial: Option<Reading>,
    games: Vec<FileGame>,
}

#[derive(Serialize, Deserialize)]
struct FileGame {
    session: String,
    index: u64,
    #[serde(flatten)]
    reading: Reading,
}

#[derive(Debug)]
pub struct SavedRatings {
    path: PathBuf,
    initial: Option<Reading>,
    after: BTreeMap<GameKey, Reading>,
}

/// Whether one mode's value in `now` follows from the one in `before` (which
/// was fresh) given the game played: it moved in the mode just played, and it
/// did not move in the other one.
fn follows(before: &Reading, now: Option<u32>, mode_duos: bool, game_duos: Option<bool>) -> bool {
    let (Some(b), Some(n), Some(game)) = (before.of(mode_duos), now, game_duos) else {
        return false;
    };
    before.fresh(mode_duos) && ((game == mode_duos) == (n != b))
}

impl SavedRatings {
    /// Loads the file; a missing file is an empty set. A file that cannot be
    /// read is an error (and is left alone), never silently replaced.
    pub fn open(dir: &Path) -> io::Result<SavedRatings> {
        fs::create_dir_all(dir)?;
        let path = dir.join(FILE_NAME);
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                return Ok(SavedRatings {
                    path,
                    initial: None,
                    after: BTreeMap::new(),
                })
            }
            Err(e) => return Err(e),
        };
        let invalid = |what: &str| io::Error::new(io::ErrorKind::InvalidData, what.to_string());
        let file: FileFormat =
            serde_json::from_str(&text).map_err(|_| invalid("unreadable game ratings file"))?;
        if file.version != FORMAT_VERSION {
            return Err(invalid("unknown game ratings file version"));
        }
        if file.initial.is_some_and(|r| !r.in_range())
            || file.games.iter().any(|g| !g.reading.in_range())
        {
            return Err(invalid("bad game ratings entry"));
        }
        let after = file
            .games
            .into_iter()
            .map(|g| {
                let key = GameKey {
                    session: g.session,
                    index: g.index,
                };
                (key, g.reading)
            })
            .collect();
        Ok(SavedRatings {
            path,
            initial: file.initial,
            after,
        })
    }

    /// The reading taken after a game; `None` asks for the one before any game.
    pub fn after(&self, game: Option<&GameKey>) -> Option<Reading> {
        match game {
            Some(key) => self.after.get(key).copied(),
            None => self.initial,
        }
    }

    pub fn len(&self) -> usize {
        self.after.len() + usize::from(self.initial.is_some())
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Saves a reading with its freshness worked out from the reading before
    /// it. True when the file changed; the same reading again writes nothing.
    /// A rating outside the game's range, or none at all, is refused.
    pub fn record(&mut self, taken: Taken) -> io::Result<bool> {
        let before = taken.after.and_then(|_| self.after(taken.previous));
        let fresh = |duos: bool| {
            let now = if duos { taken.duos } else { taken.solo };
            taken.first_of_run || before.is_some_and(|b| follows(&b, now, duos, taken.game_duos))
        };
        let reading = Reading {
            solo: taken.solo,
            duos: taken.duos,
            fresh_solo: fresh(false),
            fresh_duos: fresh(true),
        };
        if !reading.in_range() || (reading.solo.is_none() && reading.duos.is_none()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "rating out of range",
            ));
        }
        if self.after(taken.after) == Some(reading) {
            return Ok(false);
        }
        let (mut initial, mut after) = (self.initial, self.after.clone());
        match taken.after {
            Some(key) => {
                after.insert(key.clone(), reading);
            }
            None => initial = Some(reading),
        }
        self.commit(initial, after).map(|()| true)
    }

    /// Deletes every saved reading and the file (the read turned off).
    pub fn forget_all(&mut self) -> io::Result<()> {
        match fs::remove_file(&self.path) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        self.initial = None;
        self.after.clear();
        Ok(())
    }

    /// Writes the new state to disk and only then takes it, so a failed
    /// write changes nothing. The file is replaced whole (temp file, rename).
    fn commit(
        &mut self,
        initial: Option<Reading>,
        after: BTreeMap<GameKey, Reading>,
    ) -> io::Result<()> {
        let file = FileFormat {
            version: FORMAT_VERSION,
            initial,
            games: after
                .iter()
                .map(|(key, reading)| FileGame {
                    session: key.session.clone(),
                    index: key.index,
                    reading: *reading,
                })
                .collect(),
        };
        let text = serde_json::to_string_pretty(&file).map_err(io::Error::other)?;
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, text)?;
        fs::rename(&tmp, &self.path)?;
        self.initial = initial;
        self.after = after;
        Ok(())
    }

    /// The rating a game won or lost in its mode (`duos`): its reading minus
    /// the one of `previous`, the game right before it in the history
    /// (`None`: it is the first). Only when both readings exist, the earlier
    /// one is fresh and the value moved; otherwise unknown, never a guess.
    pub fn change(&self, game: &GameKey, previous: Option<&GameKey>, duos: bool) -> Option<i32> {
        let now = self.after(Some(game))?.of(duos)?;
        let before = self.after(previous).filter(|r| r.fresh(duos))?.of(duos)?;
        (now != before).then(|| now as i32 - before as i32)
    }

    /// The ratings a stretch of games started with: the fresh ones taken right
    /// before its first game (`previous`: the game before it in the history,
    /// `None` when it is the first of all), per mode (solo, duos).
    pub fn starting(&self, previous: Option<&GameKey>) -> (Option<u32>, Option<u32>) {
        let reading = self.after(previous);
        (
            reading.filter(|r| r.fresh_solo).and_then(|r| r.solo),
            reading.filter(|r| r.fresh_duos).and_then(|r| r.duos),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tl-game-ratings-{tag}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn key(index: u64) -> GameKey {
        GameKey {
            session: "Hearthstone_2026_10_14_20_00_00".into(),
            index,
        }
    }

    /// A reading after game `index` (`None`: before any game), the game being
    /// Solo, with the game before it being `index - 1`.
    fn take(saved: &mut SavedRatings, index: Option<u64>, solo: u32, duos: u32, first: bool) {
        take_mode(saved, index, Some(false), solo, duos, first);
    }

    fn take_mode(
        saved: &mut SavedRatings,
        index: Option<u64>,
        game_duos: Option<bool>,
        solo: u32,
        duos: u32,
        first: bool,
    ) {
        let after = index.map(key);
        let previous = index
            .and_then(|i| i.checked_sub(1))
            .filter(|i| *i > 0)
            .map(key);
        saved
            .record(Taken {
                after: after.as_ref(),
                previous: previous.as_ref(),
                game_duos,
                first_of_run: first,
                solo: Some(solo),
                duos: Some(duos),
            })
            .unwrap();
    }

    #[test]
    fn a_reading_is_saved_per_game_and_survives_a_restart() {
        let d = dir("save");
        let mut saved = SavedRatings::open(&d).unwrap();
        assert!(saved.is_empty());
        take(&mut saved, None, 6400, 5800, true);
        take(&mut saved, Some(1), 6420, 5800, false);
        take(&mut saved, Some(1), 6425, 5800, false);
        let back = SavedRatings::open(&d).unwrap();
        assert_eq!(back.len(), 2);
        assert_eq!(back.after(None).unwrap().solo, Some(6400));
        assert_eq!(
            back.after(Some(&key(1))).unwrap().solo,
            Some(6425),
            "a later read after the same game replaces it"
        );
        assert_eq!(back.after(Some(&key(2))), None);
    }

    #[test]
    fn the_same_reading_again_writes_nothing() {
        let mut saved = SavedRatings::open(&dir("same")).unwrap();
        let taken = Taken {
            after: Some(&key(1)),
            previous: None,
            game_duos: Some(false),
            first_of_run: true,
            solo: Some(6400),
            duos: None,
        };
        assert!(saved.record(taken).unwrap());
        assert!(!saved.record(taken).unwrap());
    }

    #[test]
    fn the_change_is_the_difference_to_the_game_right_before() {
        let mut saved = SavedRatings::open(&dir("change")).unwrap();
        take(&mut saved, None, 6400, 5800, true);
        take(&mut saved, Some(1), 6440, 5800, false);
        take_mode(&mut saved, Some(2), Some(true), 6440, 5760, false);
        assert_eq!(saved.change(&key(1), None, false), Some(40));
        // A Duos game: the Solo value did not move, the Duos one did.
        assert_eq!(saved.change(&key(2), Some(&key(1)), true), Some(-40));
        // The mode that did not move says nothing: it may not be updated yet.
        assert_eq!(saved.change(&key(2), Some(&key(1)), false), None);
    }

    #[test]
    fn a_reading_that_did_not_move_in_its_mode_is_not_fresh() {
        let mut saved = SavedRatings::open(&dir("stale")).unwrap();
        take(&mut saved, None, 6400, 5800, true);
        // The game shows the old rating after game 1: not fresh.
        take(&mut saved, Some(1), 6400, 5800, false);
        assert!(!saved.after(Some(&key(1))).unwrap().fresh_solo);
        take(&mut saved, Some(2), 6430, 5800, false);
        // Game 2 moved, but game 1's reading may be stale: no change, and the
        // freshness does not come back by itself.
        assert_eq!(saved.change(&key(2), Some(&key(1)), false), None);
        assert!(!saved.after(Some(&key(2))).unwrap().fresh_solo);
        assert_eq!(saved.starting(Some(&key(1))), (None, Some(5800)));
    }

    #[test]
    fn a_reading_that_skips_a_game_is_not_fresh() {
        // Games 4 and 5 passed with no reading after game 4: game 5's reading
        // may still show game 4's rating, and the change would hold two games.
        let mut saved = SavedRatings::open(&dir("skip")).unwrap();
        take(&mut saved, Some(3), 6400, 5800, true);
        take(&mut saved, Some(5), 6430, 5800, false);
        assert!(!saved.after(Some(&key(5))).unwrap().fresh_solo);
        take(&mut saved, Some(6), 6470, 5800, false);
        assert_eq!(saved.change(&key(6), Some(&key(5)), false), None);
    }

    #[test]
    fn a_game_not_in_the_counted_history_keeps_its_own_reading() {
        // Game 6 was played with the reconnect dev tool: it has its own
        // reading, so game 5's is not overwritten by it, and game 7's change
        // is measured against game 6.
        let mut saved = SavedRatings::open(&dir("dev")).unwrap();
        take(&mut saved, Some(4), 6400, 5800, true);
        take(&mut saved, Some(5), 6440, 5800, false);
        take(&mut saved, Some(6), 6420, 5800, false);
        take(&mut saved, Some(7), 6450, 5800, false);
        assert_eq!(saved.change(&key(5), Some(&key(4)), false), Some(40));
        assert_eq!(saved.change(&key(7), Some(&key(6)), false), Some(30));
    }

    #[test]
    fn a_session_starts_with_the_fresh_reading_before_its_first_game() {
        let mut saved = SavedRatings::open(&dir("start")).unwrap();
        take(&mut saved, None, 6400, 5800, true);
        take(&mut saved, Some(1), 6400, 5800, false);
        assert_eq!(saved.starting(None), (Some(6400), Some(5800)));
        assert_eq!(
            saved.starting(Some(&key(1))),
            (None, Some(5800)),
            "solo may be stale"
        );
        assert_eq!(saved.starting(Some(&key(7))), (None, None));
    }

    #[test]
    fn a_rating_outside_the_game_range_is_refused() {
        let mut saved = SavedRatings::open(&dir("range")).unwrap();
        for bad in [0, 30_001] {
            let taken = Taken {
                after: None,
                previous: None,
                game_duos: None,
                first_of_run: true,
                solo: Some(bad),
                duos: Some(5800),
            };
            assert!(saved.record(taken).is_err());
        }
        let none = Taken {
            after: None,
            previous: None,
            game_duos: None,
            first_of_run: true,
            solo: None,
            duos: None,
        };
        assert!(saved.record(none).is_err());
        assert!(saved.is_empty());
    }

    #[test]
    fn turning_the_read_off_deletes_the_file_and_a_bad_file_is_left_alone() {
        let d = dir("forget");
        let mut saved = SavedRatings::open(&d).unwrap();
        take(&mut saved, Some(1), 6440, 5800, true);
        assert!(d.join(FILE_NAME).exists());
        saved.forget_all().unwrap();
        assert!(!d.join(FILE_NAME).exists());
        assert!(saved.is_empty());
        saved.forget_all().unwrap();
        fs::write(d.join(FILE_NAME), "not json").unwrap();
        assert!(SavedRatings::open(&d).is_err());
        assert_eq!(fs::read_to_string(d.join(FILE_NAME)).unwrap(), "not json");
    }
}
