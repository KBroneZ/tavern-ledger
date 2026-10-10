//! The switch for reading two values from the game (D-063), and when the
//! reader runs. Off by default: with it off, nothing here touches the game.
//!
//! When on, the reader (`game-memory` crate, read-only rights) is asked for:
//! - the lobby's five tribes while a Battlegrounds game is in progress (from
//!   hero select), every few seconds for about a minute, until a read passes
//!   its checks; a failed check stops it for that game ("unsupported game
//!   version") and the app keeps its log-based tribes;
//! - the player's own rating each time the Battlegrounds lobby screen opens.
//!
//! The values stay in memory on this PC: they are not saved with the games,
//! not put in any report and not uploaded.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use game_memory::{Rating, ReadFailure};
use serde::{Deserialize, Serialize};
use tracker::game_values::{OwnRating, ReaderStatus, CHECKED_BUILDS};
use tracker::provenance::Source;
use tracker::scene::Scene;
use tracker::store::GameKey;

use crate::overlay::lock;

const FILE: &str = "game_memory.json";
const FORMAT_VERSION: u32 = 1;
/// Ticks (one per poll, about a second) between two reads of the tribes.
const TRIBES_EVERY: u32 = 3;
/// Reads of the tribes per game: about a minute, since the game fills them
/// some seconds into hero select.
const TRIBES_TRIES: u32 = 20;
/// Reads of the rating per visit to the lobby screen.
const RATING_TRIES: u32 = 10;

/// Asks the game for the two values. The real one opens the game read-only;
/// tests use a fake that counts calls.
pub trait Reader {
    fn lobby_tribes(&self) -> Result<Vec<String>, ReadFailure>;
    fn own_rating(&self) -> Result<Rating, ReadFailure>;
}

pub struct GameReader;

impl Reader for GameReader {
    fn lobby_tribes(&self) -> Result<Vec<String>, ReadFailure> {
        game_memory::lobby_tribes().map(|t| t.into_iter().map(String::from).collect())
    }

    fn own_rating(&self) -> Result<Rating, ReadFailure> {
        game_memory::own_rating()
    }
}

#[derive(Serialize, Deserialize)]
struct FileFormat {
    version: u32,
    enabled: bool,
}

#[derive(Default)]
struct State {
    enabled: bool,
    /// The game the tribes belong to.
    game: Option<GameKey>,
    tribes: Option<Vec<String>>,
    tribes_status: ReaderStatus,
    tribes_tries: u32,
    tribes_done: bool,
    ticks: u32,
    rating: Option<Rating>,
    rating_status: ReaderStatus,
    rating_tries: u32,
    /// Read (or given up) on this visit to the lobby screen.
    rating_done: bool,
    /// The last status of any read, for the settings page.
    status: ReaderStatus,
}

#[derive(Default)]
pub struct GameMemory {
    state: Mutex<State>,
    path: Mutex<Option<PathBuf>>,
    warning: Mutex<Option<String>>,
}

/// What the settings page shows.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct View {
    pub enabled: bool,
    /// `off`, `ok`, `waiting`, `unsupported_version`, `not_running`,
    /// `no_access`.
    pub status: ReaderStatus,
    pub tribes_status: ReaderStatus,
    pub rating: OwnRating,
    /// `game`: the label "from the game". The switch's words and the risk
    /// paragraph are the window's (`settings.memory.*`, English and Spanish).
    pub source: Source,
    pub checked_builds: &'static [i64],
    /// Set when the choice could not be saved or read.
    pub warning: Option<String>,
}

fn status_of(e: &ReadFailure) -> ReaderStatus {
    match e {
        ReadFailure::NotRunning => ReaderStatus::NotRunning,
        ReadFailure::NoAccess => ReaderStatus::NoAccess,
        ReadFailure::NotYet => ReaderStatus::Waiting,
        ReadFailure::Unsupported(_) => ReaderStatus::UnsupportedVersion,
    }
}

impl GameMemory {
    /// Reads the saved choice. No file: off. A file that cannot be read:
    /// off, and says so.
    pub fn load(&self, data_dir: &Path) -> Result<(), String> {
        let path = data_dir.join(FILE);
        *lock(&self.path) = Some(path.clone());
        let enabled = match std::fs::read_to_string(&path) {
            Ok(text) => match serde_json::from_str::<FileFormat>(&text) {
                Ok(f) if f.version == FORMAT_VERSION => f.enabled,
                _ => {
                    let msg = "The setting for reading from the game could not be read; it is off.";
                    *lock(&self.warning) = Some(msg.into());
                    return Err(msg.into());
                }
            },
            Err(e) if e.kind() == io::ErrorKind::NotFound => false,
            Err(e) => {
                let msg = format!(
                    "The setting for reading from the game could not be read ({:?}); it is off.",
                    e.kind()
                );
                *lock(&self.warning) = Some(msg.clone());
                return Err(msg);
            }
        };
        self.apply(enabled);
        Ok(())
    }

    /// Turns it off for this run without saving (after an internal error).
    pub fn stop_for_run(&self) {
        self.apply(false);
    }

    /// Turns it on or off and saves the choice. Turning it off forgets every
    /// value read. The choice holds for this run even if saving fails.
    pub fn set_enabled(&self, on: bool) -> Result<(), String> {
        self.apply(on);
        let failed = |kind: io::ErrorKind| {
            format!("The setting could not be saved ({kind:?}); it will be lost after a restart.")
        };
        let result = match lock(&self.path).clone() {
            None => Err(failed(io::ErrorKind::NotFound)),
            Some(path) => serde_json::to_string_pretty(&FileFormat {
                version: FORMAT_VERSION,
                enabled: on,
            })
            .map_err(|_| failed(io::ErrorKind::InvalidData))
            .and_then(|text| write_whole(&path, &text).map_err(|e| failed(e.kind()))),
        };
        *lock(&self.warning) = result.clone().err();
        result
    }

    fn apply(&self, on: bool) {
        let mut s = lock(&self.state);
        *s = State {
            enabled: on,
            status: if on {
                ReaderStatus::Waiting
            } else {
                ReaderStatus::Off
            },
            tribes_status: if on {
                ReaderStatus::Waiting
            } else {
                ReaderStatus::Off
            },
            rating_status: if on {
                ReaderStatus::Waiting
            } else {
                ReaderStatus::Off
            },
            ..State::default()
        };
    }

    /// Called once per poll with the game in progress and the scene. Asks
    /// the reader only when the switch is on, and only when there is
    /// something to read.
    pub fn tick(&self, reader: &dyn Reader, game: Option<&GameKey>, scene: Option<Scene>) {
        let mut s = lock(&self.state);
        if !s.enabled {
            return;
        }
        s.ticks = s.ticks.wrapping_add(1);
        tick_tribes(&mut s, reader, game);
        tick_rating(&mut s, reader, scene);
    }

    /// The tribes read for `game` and the reader's status for them.
    pub fn tribes_for(&self, game: Option<&GameKey>) -> (Option<Vec<String>>, ReaderStatus) {
        let s = lock(&self.state);
        if !s.enabled {
            return (None, ReaderStatus::Off);
        }
        if game.is_none() || s.game.as_ref() != game {
            return (None, ReaderStatus::Waiting);
        }
        (s.tribes.clone(), s.tribes_status)
    }

    pub fn view(&self) -> View {
        let s = lock(&self.state);
        View {
            enabled: s.enabled,
            status: s.status,
            tribes_status: s.tribes_status,
            rating: OwnRating {
                solo: s.rating.and_then(|r| r.solo),
                duos: s.rating.and_then(|r| r.duos),
                status: s.rating_status,
            },
            source: Source::Game,
            checked_builds: CHECKED_BUILDS,
            warning: lock(&self.warning).clone(),
        }
    }
}

fn tick_tribes(s: &mut State, reader: &dyn Reader, game: Option<&GameKey>) {
    let Some(game) = game else {
        return;
    };
    if s.game.as_ref() != Some(game) {
        s.game = Some(game.clone());
        s.tribes = None;
        s.tribes_status = ReaderStatus::Waiting;
        s.tribes_tries = 0;
        s.tribes_done = false;
        s.ticks = 0;
    }
    if s.tribes_done || !s.ticks.is_multiple_of(TRIBES_EVERY) {
        return;
    }
    s.tribes_tries += 1;
    let result = reader.lobby_tribes();
    s.tribes_status = match &result {
        Ok(_) => ReaderStatus::Ok,
        Err(e) => status_of(e),
    };
    s.status = s.tribes_status;
    match result {
        Ok(tribes) => {
            s.tribes = Some(tribes);
            s.tribes_done = true;
        }
        Err(ReadFailure::NotYet) | Err(ReadFailure::NotRunning) => {}
        Err(_) => s.tribes_done = true,
    }
    if !s.tribes_done && s.tribes_tries >= TRIBES_TRIES {
        // Never filled in a minute of hero select: the structure is not
        // the one the reader knows.
        s.tribes_done = true;
        if s.tribes_status == ReaderStatus::Waiting {
            s.tribes_status = ReaderStatus::UnsupportedVersion;
            s.status = s.tribes_status;
        }
    }
}

fn tick_rating(s: &mut State, reader: &dyn Reader, scene: Option<Scene>) {
    if scene != Some(Scene::BattlegroundsLobby) {
        s.rating_done = false;
        s.rating_tries = 0;
        return;
    }
    if s.rating_done {
        return;
    }
    s.rating_tries += 1;
    let result = reader.own_rating();
    s.rating_status = match &result {
        Ok(_) => ReaderStatus::Ok,
        Err(e) => status_of(e),
    };
    s.status = s.rating_status;
    match result {
        Ok(rating) => {
            s.rating = Some(rating);
            s.rating_done = true;
        }
        Err(ReadFailure::NotYet) | Err(ReadFailure::NotRunning) => {
            s.rating_done = s.rating_tries >= RATING_TRIES;
        }
        Err(_) => {
            // A value that failed its checks is never kept.
            s.rating = None;
            s.rating_done = true;
        }
    }
}

/// Written whole to a side file first, so a crash never leaves half a file.
fn write_whole(path: &Path, text: &str) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let side = path.with_extension("json.tmp");
    std::fs::write(&side, text)?;
    std::fs::rename(&side, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    struct FakeReader {
        tribes: RefCell<Vec<Result<Vec<String>, ReadFailure>>>,
        rating: RefCell<Vec<Result<Rating, ReadFailure>>>,
        calls: Cell<u32>,
    }

    impl FakeReader {
        fn new(
            tribes: Vec<Result<Vec<String>, ReadFailure>>,
            rating: Vec<Result<Rating, ReadFailure>>,
        ) -> FakeReader {
            FakeReader {
                tribes: RefCell::new(tribes),
                rating: RefCell::new(rating),
                calls: Cell::new(0),
            }
        }
    }

    impl Reader for FakeReader {
        fn lobby_tribes(&self) -> Result<Vec<String>, ReadFailure> {
            self.calls.set(self.calls.get() + 1);
            let mut list = self.tribes.borrow_mut();
            if list.len() > 1 {
                list.remove(0)
            } else {
                list[0].clone()
            }
        }

        fn own_rating(&self) -> Result<Rating, ReadFailure> {
            self.calls.set(self.calls.get() + 1);
            let mut list = self.rating.borrow_mut();
            if list.len() > 1 {
                list.remove(0)
            } else {
                list[0].clone()
            }
        }
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("tl-gm-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn key(index: u64) -> GameKey {
        GameKey {
            session: "Hearthstone_2026_10_11_01_10_48".into(),
            index,
        }
    }

    fn lobby() -> Vec<String> {
        ["BEAST", "DRAGON", "MURLOC", "PIRATE", "UNDEAD"]
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    fn on(tag: &str) -> GameMemory {
        let gm = GameMemory::default();
        gm.load(&temp_dir(tag)).unwrap();
        gm.set_enabled(true).unwrap();
        gm
    }

    fn ticks(
        gm: &GameMemory,
        reader: &dyn Reader,
        game: Option<&GameKey>,
        scene: Option<Scene>,
        n: u32,
    ) {
        for _ in 0..n {
            gm.tick(reader, game, scene);
        }
    }

    const RATING: Rating = Rating {
        solo: Some(6543),
        duos: None,
    };

    #[test]
    fn it_is_off_until_the_user_turns_it_on() {
        let gm = GameMemory::default();
        gm.load(&temp_dir("default")).unwrap();
        assert!(!gm.view().enabled);
        assert_eq!(gm.view().status, ReaderStatus::Off);
    }

    #[test]
    fn with_it_off_the_game_is_never_asked() {
        let gm = GameMemory::default();
        gm.load(&temp_dir("never")).unwrap();
        let reader = FakeReader::new(vec![Ok(lobby())], vec![Ok(RATING)]);
        ticks(
            &gm,
            &reader,
            Some(&key(1)),
            Some(Scene::BattlegroundsLobby),
            50,
        );
        ticks(&gm, &reader, Some(&key(1)), Some(Scene::Game), 50);
        assert_eq!(reader.calls.get(), 0);
        assert_eq!(gm.tribes_for(Some(&key(1))), (None, ReaderStatus::Off));
        // Off by default even for a value of GameMemory never loaded.
        let unloaded = GameMemory::default();
        unloaded.tick(&reader, Some(&key(1)), Some(Scene::BattlegroundsLobby));
        assert_eq!(reader.calls.get(), 0);
    }

    #[test]
    fn the_choice_is_kept_for_the_next_start_and_off_forgets_values() {
        let dir = temp_dir("kept");
        let gm = GameMemory::default();
        gm.load(&dir).unwrap();
        gm.set_enabled(true).unwrap();
        let again = GameMemory::default();
        again.load(&dir).unwrap();
        assert!(again.view().enabled);
        let reader = FakeReader::new(vec![Ok(lobby())], vec![Ok(RATING)]);
        ticks(
            &again,
            &reader,
            Some(&key(1)),
            Some(Scene::BattlegroundsLobby),
            3,
        );
        assert!(again.tribes_for(Some(&key(1))).0.is_some());
        again.set_enabled(false).unwrap();
        assert_eq!(again.tribes_for(Some(&key(1))), (None, ReaderStatus::Off));
        assert_eq!(again.view().rating.solo, None);
        let third = GameMemory::default();
        third.load(&dir).unwrap();
        assert!(!third.view().enabled);
    }

    #[test]
    fn a_broken_setting_file_leaves_it_off_and_says_so() {
        let dir = temp_dir("broken");
        std::fs::write(dir.join(FILE), "{ not json").unwrap();
        let gm = GameMemory::default();
        assert!(gm.load(&dir).is_err());
        assert!(!gm.view().enabled);
        assert!(gm.view().warning.is_some());
        std::fs::write(dir.join(FILE), r#"{"version": 9, "enabled": true}"#).unwrap();
        let gm = GameMemory::default();
        assert!(gm.load(&dir).is_err());
        assert!(!gm.view().enabled);
    }

    #[test]
    fn tribes_are_read_during_a_game_and_kept_for_it() {
        let gm = on("tribes");
        let reader = FakeReader::new(
            vec![Err(ReadFailure::NotYet), Ok(lobby())],
            vec![Ok(RATING)],
        );
        // Nothing in progress: nothing read.
        ticks(&gm, &reader, None, Some(Scene::Game), 9);
        assert_eq!(reader.calls.get(), 0);
        ticks(&gm, &reader, Some(&key(1)), Some(Scene::Game), 3);
        assert_eq!(gm.tribes_for(Some(&key(1))), (None, ReaderStatus::Waiting));
        ticks(&gm, &reader, Some(&key(1)), Some(Scene::Game), 3);
        assert_eq!(
            gm.tribes_for(Some(&key(1))),
            (Some(lobby()), ReaderStatus::Ok)
        );
        let calls = reader.calls.get();
        ticks(&gm, &reader, Some(&key(1)), Some(Scene::Game), 30);
        assert_eq!(reader.calls.get(), calls, "read once per game");
        // Another game starts fresh.
        assert_eq!(gm.tribes_for(Some(&key(2))), (None, ReaderStatus::Waiting));
        ticks(&gm, &reader, Some(&key(2)), Some(Scene::Game), 3);
        assert_eq!(gm.tribes_for(Some(&key(2))).1, ReaderStatus::Ok);
    }

    #[test]
    fn an_unsupported_version_stops_reading_for_that_game() {
        let gm = on("unsupported");
        let reader = FakeReader::new(
            vec![Err(ReadFailure::Unsupported(
                "class GameState not found".into(),
            ))],
            vec![Ok(RATING)],
        );
        ticks(&gm, &reader, Some(&key(1)), Some(Scene::Game), 60);
        assert_eq!(reader.calls.get(), 1);
        assert_eq!(
            gm.tribes_for(Some(&key(1))),
            (None, ReaderStatus::UnsupportedVersion)
        );
        assert_eq!(gm.view().status, ReaderStatus::UnsupportedVersion);
    }

    #[test]
    fn no_access_is_not_retried() {
        let gm = on("noaccess");
        let reader = FakeReader::new(vec![Err(ReadFailure::NoAccess)], vec![Ok(RATING)]);
        ticks(&gm, &reader, Some(&key(1)), Some(Scene::Game), 60);
        assert_eq!(reader.calls.get(), 1);
        assert_eq!(gm.tribes_for(Some(&key(1))).1, ReaderStatus::NoAccess);
    }

    #[test]
    fn tribes_never_filled_give_up_after_a_minute_as_unsupported() {
        let gm = on("never-filled");
        let reader = FakeReader::new(vec![Err(ReadFailure::NotYet)], vec![Ok(RATING)]);
        ticks(&gm, &reader, Some(&key(1)), Some(Scene::Game), 200);
        assert_eq!(reader.calls.get(), TRIBES_TRIES);
        assert_eq!(
            gm.tribes_for(Some(&key(1))).1,
            ReaderStatus::UnsupportedVersion
        );
    }

    #[test]
    fn the_rating_is_read_once_per_visit_to_the_lobby_screen() {
        let gm = on("rating");
        let reader = FakeReader::new(vec![Ok(lobby())], vec![Ok(RATING)]);
        ticks(&gm, &reader, None, Some(Scene::Menu), 5);
        assert_eq!(reader.calls.get(), 0);
        ticks(&gm, &reader, None, Some(Scene::BattlegroundsLobby), 5);
        assert_eq!(reader.calls.get(), 1);
        let view = gm.view();
        assert_eq!((view.rating.solo, view.rating.duos), (Some(6543), None));
        assert_eq!(view.rating.status, ReaderStatus::Ok);
        assert_eq!(view.source, Source::Game);
        ticks(&gm, &reader, None, Some(Scene::Game), 1);
        ticks(&gm, &reader, None, Some(Scene::BattlegroundsLobby), 5);
        assert_eq!(reader.calls.get(), 2, "read again after a game");
    }

    #[test]
    fn a_rating_that_fails_its_checks_is_dropped() {
        let gm = on("rating-bad");
        let reader = FakeReader::new(
            vec![Ok(lobby())],
            vec![
                Ok(RATING),
                Err(ReadFailure::Unsupported("rating out of range".into())),
            ],
        );
        ticks(&gm, &reader, None, Some(Scene::BattlegroundsLobby), 1);
        assert_eq!(gm.view().rating.solo, Some(6543));
        ticks(&gm, &reader, None, Some(Scene::Game), 1);
        ticks(&gm, &reader, None, Some(Scene::BattlegroundsLobby), 1);
        let view = gm.view();
        assert_eq!(view.rating.solo, None);
        assert_eq!(view.rating.status, ReaderStatus::UnsupportedVersion);
    }
}
