//! The switch for reading two values from the game (D-063), and when the
//! reader runs. Off by default: with it off, nothing here touches the game.
//!
//! When on, the reader (`game-memory` crate, read-only rights) is asked for:
//! - the lobby's five tribes while a Battlegrounds game is in progress (from
//!   hero select), every few seconds for about a minute, until two reads in
//!   a row pass their checks and agree, and are not the last game's five
//!   again (the game may still hold those as a game starts); only on a game
//!   build the reader was checked on (any other build: not read at all); a
//!   failed check, or the tavern contradicting the tribes, stops it for that
//!   game ("unsupported game version") and the app keeps its log-based
//!   tribes;
//! - the player's own rating while the Battlegrounds lobby screen is open
//!   (T-337): every few seconds for about a minute per visit, only when the
//!   running client's build is one the reader was checked on (from the
//!   session's log, also before its first game), taken only when two reads in
//!   a row agree. The change since the last game is the new rating minus the
//!   one taken before that game, for that game's mode; it is shown only when
//!   exactly one game was played between the two, never a guess.
//!
//! The rating keeps being read every half minute for the rest of the visit,
//! since the game can update it later than that (T-342).
//!
//! The values stay on this PC: they are not put in any report and not
//! uploaded. The last rating taken is written to a file of its own
//! (`game_rating.json`, with the build and the newest game it includes), so
//! the change after the next game is known across a restart of the app
//! (T-342, D-072), and each game's tribes, once checked, to
//! `game_tribes.json` next to the history (T-540, D-104, `saved_tribes.rs`);
//! turning the switch off deletes both. The tribes are read whether or not
//! the overlay is on, since the recap and the stats use them too (D-106).

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
/// The last rating taken, for the change across a restart of the app.
const RATING_FILE: &str = "game_rating.json";
const RATING_FORMAT_VERSION: u32 = 1;
const RATING_SAVE_FAILED: &str = "The last rating could not be saved";
/// Ticks (one per poll, about a second) between two reads of the tribes.
const TRIBES_EVERY: u32 = 3;
/// Reads of the tribes per game: about a minute, since the game fills them
/// some seconds into hero select.
const TRIBES_TRIES: u32 = 20;
/// Ticks between two reads of the rating on the lobby screen.
const RATING_EVERY: u32 = 3;
/// Reads of the rating per visit to the lobby screen: about a minute, since
/// the game may update the rating some seconds after the lobby opens.
const RATING_TRIES: u32 = 20;
/// Ticks between two reads after that, for the rest of the visit: the game
/// can update the rating later than a minute after the lobby opens (T-342).
const RATING_SLOW_EVERY: u32 = 30;

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

/// What the app knows on one poll, for the reader.
#[derive(Clone, Copy, Debug, Default)]
pub struct Poll<'a> {
    /// The Battlegrounds game in progress, if any.
    pub game: Option<&'a GameKey>,
    /// The build of the game in progress, from its log.
    pub build: Option<i64>,
    pub scene: Option<Scene>,
    /// The running client's build, also before its first game (T-337).
    pub client_build: Option<i64>,
    /// Games finished so far in the history.
    pub finished: usize,
    /// The newest game in the history and the one before it (the same games
    /// `finished` counts), to place a rating saved by an earlier run.
    pub newest: Option<&'a GameKey>,
    pub previous: Option<&'a GameKey>,
    /// The newest game the app has seen and the one before it, the games
    /// played with the reconnect dev tool included: they move the rating too,
    /// so the rating saved with each game (T-366) follows these.
    pub newest_any: Option<&'a GameKey>,
    pub previous_any: Option<&'a GameKey>,
    /// Whether `newest_any` was Duos (`None`: none, or another mode).
    pub last_duos_any: Option<bool>,
    /// Whether the last finished game was Duos (`None`: no game, or another
    /// mode).
    pub last_duos: Option<bool>,
}

/// The rating the last agreed read took and what the app knew then, for saving
/// it with the game it follows (T-366, D-147).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TakenReading {
    /// The newest game the app had seen (`None`: before any game).
    pub after: Option<GameKey>,
    /// The game before it.
    pub previous: Option<GameKey>,
    pub game_duos: Option<bool>,
    /// The first rating taken since the app started reading.
    pub first_of_run: bool,
    pub solo: Option<u32>,
    pub duos: Option<u32>,
}

/// The rating change the last game made, in its mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct RatingChange {
    pub duos: bool,
    pub delta: i32,
}

#[derive(Serialize, Deserialize)]
struct FileFormat {
    version: u32,
    enabled: bool,
}

/// A game of the history, as saved with the rating.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct SavedGame {
    session: String,
    index: u64,
}

impl SavedGame {
    fn of(key: &GameKey) -> SavedGame {
        SavedGame {
            session: key.session.clone(),
            index: key.index,
        }
    }

    fn is(&self, key: Option<&GameKey>) -> bool {
        key.is_some_and(|k| k.session == self.session && k.index == self.index)
    }
}

/// The last rating taken (`game_rating.json`): its client build, the newest
/// game of the history then (`None`: no game yet) and whether it was known
/// to include that game (see `take_rating`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct SavedRating {
    version: u32,
    build: i64,
    solo: Option<u32>,
    duos: Option<u32>,
    after: Option<SavedGame>,
    settled: bool,
}

#[derive(Default)]
struct State {
    enabled: bool,
    /// The game the tribes belong to.
    game: Option<GameKey>,
    tribes: Option<Vec<String>>,
    /// The last game's tribes: at the start of a game the game may still
    /// hold them, so the same five are not taken for the new game.
    last_tribes: Option<Vec<String>>,
    /// A read that passed, waiting for the next read to give the same.
    pending: Option<Vec<String>>,
    tribes_status: ReaderStatus,
    tribes_tries: u32,
    tribes_done: bool,
    ticks: u32,
    rating: Option<Rating>,
    rating_status: ReaderStatus,
    rating_tries: u32,
    /// Ticks on this visit to the lobby screen.
    rating_ticks: u32,
    /// Given up on this visit to the lobby screen.
    rating_done: bool,
    /// A read that passed, waiting for the next read to give the same.
    rating_pending: Option<Rating>,
    /// Games finished when `rating` was taken.
    rating_at: Option<usize>,
    /// The client build `rating` was read on: any other build drops it.
    rating_build: Option<i64>,
    /// `rating` is known to include every game finished when it was taken
    /// (see `take_rating`); only such a rating can be the base of a change.
    rating_settled: bool,
    /// Games finished when the first rating since the switch went on (or
    /// since the build changed) was taken.
    rating_first: Option<usize>,
    /// The last rating taken before the latest game(s): value, games
    /// finished then, settled.
    rating_prior: Option<(Rating, usize, bool)>,
    rating_change: Option<RatingChange>,
    /// The newest game of the history when `rating` was taken.
    rating_after: Option<SavedGame>,
    /// `rating` was read in this run; one from the file is only a base for
    /// the change, never shown.
    rating_read: bool,
    /// What the last agreed read knew, for saving it with its game (T-366).
    rating_taken: Option<TakenReading>,
    /// The rating file's content, waiting for the first read on the lobby
    /// screen to be placed in the history.
    restored: Option<SavedRating>,
    /// What the rating file holds now.
    saved: Option<SavedRating>,
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
    /// The change the last game made, when known for sure.
    pub rating_change: Option<RatingChange>,
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

/// The settings page's one-line status. The tribes lead: a rating the
/// reader cannot map (T-337) must not say the tribes are unsupported. A
/// refused access shows from either read; before any game, the rating read
/// says whether the game runs.
fn overall(tribes: ReaderStatus, rating: ReaderStatus) -> ReaderStatus {
    use ReaderStatus::*;
    if tribes == NoAccess || rating == NoAccess {
        return NoAccess;
    }
    match (tribes, rating) {
        (Waiting, NotRunning) => NotRunning,
        (Waiting, Ok) => Ok,
        (t, _) => t,
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
        if enabled {
            let saved = read_saved_rating(&data_dir.join(RATING_FILE));
            let mut s = lock(&self.state);
            s.restored = saved.clone();
            s.saved = saved;
        }
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
        if !on {
            self.delete_saved_rating();
        }
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

    /// Called once per poll with the game in progress, its build from the
    /// log and the scene. Asks the reader only when the switch is on, and
    /// only when there is something to read.
    pub fn tick(&self, reader: &dyn Reader, poll: &Poll) {
        let mut s = lock(&self.state);
        if !s.enabled {
            return;
        }
        s.ticks = s.ticks.wrapping_add(1);
        tick_tribes(&mut s, reader, poll.game, poll.build);
        tick_rating(&mut s, reader, poll);
        self.save_rating(&mut s);
    }

    fn rating_path(&self) -> Option<PathBuf> {
        lock(&self.path)
            .as_ref()
            .map(|p| p.with_file_name(RATING_FILE))
    }

    /// Writes the rating file when the rating taken changed and deletes it
    /// when the rating was dropped. A failed write is said in the warning.
    fn save_rating(&self, s: &mut State) {
        if s.restored.is_some() {
            return; // not placed yet: the file stays as it is
        }
        let now = match (s.rating, s.rating_build) {
            (Some(r), Some(build)) => Some(SavedRating {
                version: RATING_FORMAT_VERSION,
                build,
                solo: r.solo,
                duos: r.duos,
                after: s.rating_after.clone(),
                settled: s.rating_settled,
            }),
            _ => None,
        };
        if now == s.saved {
            return;
        }
        let Some(path) = self.rating_path() else {
            return;
        };
        let result = match &now {
            Some(saved) => serde_json::to_string_pretty(saved)
                .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))
                .and_then(|text| write_whole(&path, &text)),
            None => match std::fs::remove_file(&path) {
                Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
                _ => Ok(()),
            },
        };
        match result {
            Ok(()) => {
                // Only this warning goes away: the switch's own stays.
                let mut warning = lock(&self.warning);
                if warning
                    .as_deref()
                    .is_some_and(|w| w.starts_with(RATING_SAVE_FAILED))
                {
                    *warning = None;
                }
            }
            Err(e) => {
                *lock(&self.warning) = Some(format!(
                    "{RATING_SAVE_FAILED} ({:?}); the change after a restart may be missing.",
                    e.kind()
                ));
            }
        }
        // Tried once per change, not again every poll.
        s.saved = now;
    }

    fn delete_saved_rating(&self) {
        if let Some(path) = self.rating_path() {
            let _ = std::fs::remove_file(path);
        }
    }

    /// The tavern contradicted the tribes read for `game`: they are dropped
    /// for the rest of that game, whatever the tavern shows later.
    pub fn refuse(&self, game: &GameKey) {
        let mut s = lock(&self.state);
        if s.enabled && s.game.as_ref() == Some(game) {
            s.tribes = None;
            s.pending = None;
            s.tribes_status = ReaderStatus::UnsupportedVersion;
            s.tribes_done = true;
        }
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

    /// The rating the last agreed read took, for saving with the game it
    /// follows (T-366, D-147): the newest game of the history then (`None`:
    /// before any game) and the two ratings. Only a rating read in this run
    /// on a checked build; none while the switch is off.
    pub fn taken_reading(&self) -> Option<TakenReading> {
        let s = lock(&self.state);
        s.rating_taken
            .clone()
            .filter(|_| s.enabled && s.rating_read)
    }

    pub fn view(&self) -> View {
        let s = lock(&self.state);
        View {
            enabled: s.enabled,
            status: overall(s.tribes_status, s.rating_status),
            tribes_status: s.tribes_status,
            rating: OwnRating {
                solo: s.rating.filter(|_| s.rating_read).and_then(|r| r.solo),
                duos: s.rating.filter(|_| s.rating_read).and_then(|r| r.duos),
                status: s.rating_status,
            },
            rating_change: s.rating_change,
            source: Source::Game,
            checked_builds: CHECKED_BUILDS,
            warning: lock(&self.warning).clone(),
        }
    }
}

fn tick_tribes(s: &mut State, reader: &dyn Reader, game: Option<&GameKey>, build: Option<i64>) {
    let Some(game) = game else {
        return;
    };
    if s.game.as_ref() != Some(game) {
        s.game = Some(game.clone());
        if let Some(last) = s.tribes.take() {
            s.last_tribes = Some(last);
        }
        s.pending = None;
        s.tribes_status = ReaderStatus::Waiting;
        s.tribes_tries = 0;
        s.tribes_done = false;
        s.ticks = 0;
    }
    if s.tribes_done || !s.ticks.is_multiple_of(TRIBES_EVERY) {
        return;
    }
    // The path is trusted only on the builds it was checked on: on any other
    // build the game is not even opened (D-063's fail-safe rule).
    match build {
        None => return,
        Some(b) if !CHECKED_BUILDS.contains(&b) => {
            s.tribes_status = ReaderStatus::UnsupportedVersion;
            s.tribes_done = true;
            return;
        }
        Some(_) => {}
    }
    s.tribes_tries += 1;
    // Taken only when two reads in a row agree, and not the last game's five.
    let result = match reader.lobby_tribes() {
        Ok(tribes) if s.last_tribes.as_ref() == Some(&tribes) => {
            s.pending = None;
            Err(ReadFailure::NotYet)
        }
        Ok(tribes) if s.pending.as_ref() == Some(&tribes) => Ok(tribes),
        Ok(tribes) => {
            s.pending = Some(tribes);
            Err(ReadFailure::NotYet)
        }
        Err(e) => {
            s.pending = None;
            Err(e)
        }
    };
    s.tribes_status = match &result {
        Ok(_) => ReaderStatus::Ok,
        Err(e) => status_of(e),
    };
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
        }
    }
}

fn tick_rating(s: &mut State, reader: &dyn Reader, poll: &Poll) {
    // A rating belongs to one client build: when the build changes or is
    // no longer known, nothing read on the old one is kept.
    if s.rating_build.is_some() && s.rating_build != poll.client_build {
        forget_rating(s);
        // An unknown build (the game closed, a new session not read yet) is
        // not a new build: the saved base stays in its file and is placed
        // again, against the build, at the next read (T-342).
        if poll.client_build.is_none() {
            s.restored = s.saved.clone();
        }
    }
    // A change belongs to the game that made it: one more game finished and
    // it no longer says anything about the last game.
    if s.rating_at != Some(poll.finished) {
        s.rating_change = None;
    }
    if poll.scene != Some(Scene::BattlegroundsLobby) {
        s.rating_done = false;
        s.rating_tries = 0;
        s.rating_ticks = 0;
        s.rating_pending = None;
        return;
    }
    if s.rating_done {
        return;
    }
    s.rating_ticks = s.rating_ticks.saturating_add(1);
    let every = if s.rating_tries < RATING_TRIES {
        RATING_EVERY
    } else {
        RATING_SLOW_EVERY
    };
    if !(s.rating_ticks - 1).is_multiple_of(every) {
        return;
    }
    // Trusted only on a checked build of the running client: on any other
    // build the game is not even opened (D-063's fail-safe rule).
    let build = match poll.client_build {
        None => {
            s.rating_status = ReaderStatus::Waiting;
            return;
        }
        Some(b) if !CHECKED_BUILDS.contains(&b) => {
            s.rating_status = ReaderStatus::UnsupportedVersion;
            s.rating_done = true;
            return;
        }
        Some(b) => b,
    };
    if let Some(saved) = s.restored.take() {
        restore(s, saved, build, poll);
    }
    s.rating_tries = s.rating_tries.saturating_add(1);
    match reader.own_rating() {
        // Taken only when two reads in a row agree.
        Ok(rating) if s.rating_pending == Some(rating) => {
            s.rating_status = ReaderStatus::Ok;
            s.rating_build = Some(build);
            take_rating(s, rating, poll);
        }
        Ok(rating) => {
            s.rating_pending = Some(rating);
            if s.rating.is_none() {
                s.rating_status = ReaderStatus::Waiting;
            }
        }
        Err(e @ (ReadFailure::NotYet | ReadFailure::NotRunning)) => {
            s.rating_pending = None;
            if s.rating.is_none() {
                s.rating_status = status_of(&e);
            }
        }
        Err(e) => {
            // A value that failed its checks is never kept.
            forget_rating(s);
            s.rating_status = status_of(&e);
            s.rating_done = true;
        }
    }
}

/// Places a rating saved by an earlier run in the history: it is the base for
/// the next change only when it was read on this build and the newest game
/// then is still the newest game, or the one before it. Otherwise it is
/// dropped: never a guess.
fn restore(s: &mut State, saved: SavedRating, build: i64, poll: &Poll) {
    if s.rating.is_some() || saved.build != build {
        return;
    }
    let same = |game: Option<&GameKey>| match &saved.after {
        None => game.is_none(),
        Some(g) => g.is(game),
    };
    let at = if same(poll.newest) {
        poll.finished
    } else if saved.after.is_some() && same(poll.previous) && poll.finished > 0 {
        poll.finished - 1
    } else {
        return;
    };
    let rating = Rating {
        solo: saved.solo,
        duos: saved.duos,
    };
    if !rating_in_range(rating) {
        return;
    }
    s.rating = Some(rating);
    s.rating_at = Some(at);
    s.rating_build = Some(build);
    s.rating_settled = saved.settled;
    s.rating_first = saved.settled.then_some(at);
    s.rating_after = saved.after;
    s.rating_read = false;
}

/// The reader's own bounds (1 to 30 000), for a value from the file.
fn rating_in_range(r: Rating) -> bool {
    let ok = |v: Option<u32>| v.is_none_or(|v| (1..=30_000).contains(&v));
    (r.solo.is_some() || r.duos.is_some()) && ok(r.solo) && ok(r.duos)
}

/// The rating file, if it holds a rating of the known format.
fn read_saved_rating(path: &Path) -> Option<SavedRating> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str::<SavedRating>(&text)
        .ok()
        .filter(|r| r.version == RATING_FORMAT_VERSION)
}

/// Drops the rating and everything worked out from it.
fn forget_rating(s: &mut State) {
    s.rating = None;
    s.rating_pending = None;
    s.rating_at = None;
    s.rating_build = None;
    s.rating_settled = false;
    s.rating_first = None;
    s.rating_prior = None;
    s.rating_change = None;
    s.rating_after = None;
    s.rating_taken = None;
    s.rating_read = false;
    if s.rating_status == ReaderStatus::Ok {
        s.rating_status = ReaderStatus::Waiting;
    }
}

/// Keeps a rating two reads agreed on, and works out the change the last
/// game made.
///
/// The game may show the old rating for a moment after a game, so a rating
/// taken after a game counts as including it ("settled") only when it moved
/// in that game's mode; the first rating taken counts as settled. The change
/// is the rating now minus the settled one taken before the last game, only
/// when exactly one game finished between the two and the value moved. On
/// any doubt there is no change, never a guess.
fn take_rating(s: &mut State, rating: Rating, poll: &Poll) {
    let finished = poll.finished;
    match (s.rating, s.rating_at) {
        (Some(old), Some(at)) if at < finished => {
            s.rating_prior = Some((old, at, s.rating_settled));
        }
        (Some(_), Some(_)) => {}
        _ => s.rating_first = Some(finished),
    }
    s.rating = Some(rating);
    s.rating_at = Some(finished);
    s.rating_after = poll.newest.map(SavedGame::of);
    s.rating_read = true;
    s.rating_taken = Some(TakenReading {
        after: poll.newest_any.cloned(),
        previous: poll.previous_any.cloned(),
        game_duos: poll.last_duos_any,
        first_of_run: s.rating_first == Some(finished),
        solo: rating.solo,
        duos: rating.duos,
    });
    let pick = |r: Rating, duos: bool| if duos { r.duos } else { r.solo };
    let moved = |a: Rating, b: Rating| {
        poll.last_duos.is_some_and(|duos| {
            pick(a, duos).is_some() && pick(b, duos).is_some() && pick(a, duos) != pick(b, duos)
        })
    };
    s.rating_settled = s.rating_first == Some(finished)
        || s.rating_prior
            .is_some_and(|(prior, _, _)| moved(prior, rating));
    s.rating_change = match (s.rating_prior, poll.last_duos) {
        (Some((prior, at, true)), Some(duos)) if at + 1 == finished && moved(prior, rating) => {
            match (pick(prior, duos), pick(rating, duos)) {
                (Some(b), Some(n)) => Some(RatingChange {
                    duos,
                    delta: n as i32 - b as i32,
                }),
                _ => None,
            }
        }
        _ => None,
    };
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
            gm.tick(reader, &poll(game, BUILD, scene));
        }
    }

    const BUILD: Option<i64> = Some(253216);

    fn poll<'a>(game: Option<&'a GameKey>, build: Option<i64>, scene: Option<Scene>) -> Poll<'a> {
        Poll {
            game,
            build,
            scene,
            client_build: BUILD,
            ..Poll::default()
        }
    }

    const RATING: Rating = Rating {
        solo: Some(6543),
        duos: None,
    };

    #[test]
    fn the_reading_taken_belongs_to_the_newest_game_and_only_while_the_switch_is_on() {
        let gm = on("taken");
        assert_eq!(gm.taken_reading(), None, "nothing read yet");
        let reader = FakeReader::new(vec![Ok(lobby())], vec![Ok(RATING)]);
        let (newest, before) = (key(3), key(2));
        let poll = Poll {
            newest: Some(&newest),
            newest_any: Some(&newest),
            previous_any: Some(&before),
            last_duos_any: Some(false),
            finished: 3,
            ..poll(None, BUILD, Some(Scene::BattlegroundsLobby))
        };
        for _ in 0..6 {
            gm.tick(&reader, &poll);
        }
        let read = gm.taken_reading().expect("a rating two reads agreed on");
        assert_eq!(read.after, Some(newest));
        assert_eq!(read.previous, Some(before));
        assert_eq!(read.game_duos, Some(false));
        assert_eq!((read.solo, read.duos), (Some(6543), None));
        assert!(
            read.first_of_run,
            "the first rating taken since the read started"
        );
        gm.set_enabled(false).unwrap();
        assert_eq!(gm.taken_reading(), None, "off keeps nothing");
    }

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
        unloaded.tick(
            &reader,
            &poll(Some(&key(1)), BUILD, Some(Scene::BattlegroundsLobby)),
        );
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
            6,
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
        // One read that passes waits for the next to agree.
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
        // The same five as the last game are not taken (see below).
        assert_eq!(gm.tribes_for(Some(&key(2))), (None, ReaderStatus::Waiting));
        assert_eq!(reader.calls.get(), calls + 1, "game 2 is read again");
    }

    fn other_lobby() -> Vec<String> {
        ["ABERRATION", "DEMON", "DRAGON", "ELEMENTAL", "MECHANICAL"]
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    #[test]
    fn the_last_games_tribes_are_not_taken_for_the_next_game() {
        // At the start of a game the game may still hold the last lobby:
        // the same five again are read once more until they change.
        let gm = on("stale");
        let reader = FakeReader::new(
            vec![Ok(lobby()), Ok(lobby()), Ok(lobby()), Ok(other_lobby())],
            vec![Ok(RATING)],
        );
        ticks(&gm, &reader, Some(&key(1)), Some(Scene::Game), 6);
        assert_eq!(gm.tribes_for(Some(&key(1))).0, Some(lobby()));
        ticks(&gm, &reader, Some(&key(2)), Some(Scene::Game), 3);
        assert_eq!(gm.tribes_for(Some(&key(2))), (None, ReaderStatus::Waiting));
        ticks(&gm, &reader, Some(&key(2)), Some(Scene::Game), 6);
        assert_eq!(
            gm.tribes_for(Some(&key(2))),
            (Some(other_lobby()), ReaderStatus::Ok)
        );
    }

    #[test]
    fn the_same_lobby_for_a_whole_minute_falls_back() {
        // Two lobbies in a row can be the same (1 in 252); then the app
        // keeps its log-based tribes rather than risk the old value.
        let gm = on("same-lobby");
        let reader = FakeReader::new(vec![Ok(lobby())], vec![Ok(RATING)]);
        ticks(&gm, &reader, Some(&key(1)), Some(Scene::Game), 6);
        assert_eq!(gm.tribes_for(Some(&key(1))).0, Some(lobby()));
        ticks(&gm, &reader, Some(&key(2)), Some(Scene::Game), 200);
        assert_eq!(
            gm.tribes_for(Some(&key(2))),
            (None, ReaderStatus::UnsupportedVersion)
        );
    }

    #[test]
    fn two_reads_in_a_row_must_agree() {
        let gm = on("agree");
        let reader = FakeReader::new(
            vec![Ok(lobby()), Ok(other_lobby()), Ok(other_lobby())],
            vec![Ok(RATING)],
        );
        ticks(&gm, &reader, Some(&key(1)), Some(Scene::Game), 6);
        assert_eq!(gm.tribes_for(Some(&key(1))), (None, ReaderStatus::Waiting));
        ticks(&gm, &reader, Some(&key(1)), Some(Scene::Game), 3);
        assert_eq!(gm.tribes_for(Some(&key(1))).0, Some(other_lobby()));
    }

    #[test]
    fn a_build_the_reader_was_not_checked_on_is_never_read() {
        let gm = on("build");
        let reader = FakeReader::new(vec![Ok(lobby())], vec![Ok(RATING)]);
        for _ in 0..60 {
            gm.tick(
                &reader,
                &poll(Some(&key(1)), Some(999_999), Some(Scene::Game)),
            );
        }
        assert_eq!(reader.calls.get(), 0);
        assert_eq!(
            gm.tribes_for(Some(&key(1))),
            (None, ReaderStatus::UnsupportedVersion)
        );
        // No build from the log yet: wait, without reading.
        for _ in 0..60 {
            gm.tick(&reader, &poll(Some(&key(2)), None, Some(Scene::Game)));
        }
        assert_eq!(reader.calls.get(), 0);
        assert_eq!(gm.tribes_for(Some(&key(2))), (None, ReaderStatus::Waiting));
    }

    #[test]
    fn tribes_the_tavern_contradicted_stay_dropped_for_that_game() {
        let gm = on("refuse");
        let reader = FakeReader::new(vec![Ok(lobby())], vec![Ok(RATING)]);
        ticks(&gm, &reader, Some(&key(1)), Some(Scene::Game), 6);
        assert!(gm.tribes_for(Some(&key(1))).0.is_some());
        gm.refuse(&key(2)); // another game: no effect
        assert!(gm.tribes_for(Some(&key(1))).0.is_some());
        gm.refuse(&key(1));
        let calls = reader.calls.get();
        ticks(&gm, &reader, Some(&key(1)), Some(Scene::Game), 30);
        assert_eq!(reader.calls.get(), calls, "not read again");
        assert_eq!(
            gm.tribes_for(Some(&key(1))),
            (None, ReaderStatus::UnsupportedVersion)
        );
        assert_eq!(gm.view().status, ReaderStatus::UnsupportedVersion);
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

    fn in_lobby(
        gm: &GameMemory,
        reader: &dyn Reader,
        finished: usize,
        last_duos: Option<bool>,
        n: u32,
    ) {
        for _ in 0..n {
            gm.tick(
                reader,
                &Poll {
                    scene: Some(Scene::BattlegroundsLobby),
                    client_build: BUILD,
                    finished,
                    last_duos,
                    ..Poll::default()
                },
            );
        }
    }

    fn leave(gm: &GameMemory, reader: &dyn Reader) {
        gm.tick(reader, &poll(None, BUILD, Some(Scene::Game)));
    }

    fn rating(solo: u32, duos: u32) -> Rating {
        Rating {
            solo: Some(solo),
            duos: Some(duos),
        }
    }

    #[test]
    fn the_rating_is_read_on_the_lobby_screen_and_taken_when_two_reads_agree() {
        let gm = on("rating");
        let reader = FakeReader::new(vec![Ok(lobby())], vec![Ok(RATING)]);
        ticks(&gm, &reader, None, Some(Scene::Menu), 5);
        assert_eq!(reader.calls.get(), 0);
        in_lobby(&gm, &reader, 0, None, 1);
        assert_eq!(reader.calls.get(), 1);
        assert_eq!(gm.view().rating.solo, None, "one read is not enough");
        in_lobby(&gm, &reader, 0, None, RATING_EVERY);
        assert_eq!(reader.calls.get(), 2);
        let view = gm.view();
        assert_eq!((view.rating.solo, view.rating.duos), (Some(6543), None));
        assert_eq!(view.rating.status, ReaderStatus::Ok);
        assert_eq!(view.source, Source::Game);
        // Every few seconds for a minute, then every half minute for the
        // rest of the visit; read again at once after a game.
        let fast = RATING_EVERY * (RATING_TRIES - 2);
        in_lobby(&gm, &reader, 0, None, fast);
        assert_eq!(reader.calls.get(), RATING_TRIES);
        in_lobby(&gm, &reader, 0, None, 10 * RATING_SLOW_EVERY);
        assert_eq!(reader.calls.get(), RATING_TRIES + 10);
        leave(&gm, &reader);
        in_lobby(&gm, &reader, 1, Some(false), 1);
        assert_eq!(
            reader.calls.get(),
            RATING_TRIES + 11,
            "read again after a game"
        );
    }

    #[test]
    fn a_rating_the_game_updates_after_a_minute_still_gives_the_change() {
        let mut reads = vec![Ok(rating(6000, 7197)); 2 + RATING_TRIES as usize];
        reads.extend([Ok(rating(6000, 7272)), Ok(rating(6000, 7272))]);
        let gm = on("rating-late");
        let reader = FakeReader::new(vec![Ok(lobby())], reads);
        in_lobby(&gm, &reader, 3, Some(false), 2 * RATING_EVERY);
        leave(&gm, &reader);
        // The game shows the old value for the whole first minute.
        in_lobby(&gm, &reader, 4, Some(true), RATING_EVERY * RATING_TRIES);
        assert_eq!(gm.view().rating.duos, Some(7197));
        in_lobby(&gm, &reader, 4, Some(true), 3 * RATING_SLOW_EVERY);
        assert_eq!(gm.view().rating.duos, Some(7272));
        assert_eq!(gm.view().rating_change.map(|c| c.delta), Some(75));
    }

    fn in_lobby_after(
        gm: &GameMemory,
        reader: &dyn Reader,
        history: &[GameKey],
        last_duos: Option<bool>,
        n: u32,
    ) {
        for _ in 0..n {
            gm.tick(
                reader,
                &Poll {
                    scene: Some(Scene::BattlegroundsLobby),
                    client_build: BUILD,
                    finished: history.len(),
                    last_duos,
                    newest: history.last(),
                    previous: history.len().checked_sub(2).map(|i| &history[i]),
                    ..Poll::default()
                },
            );
        }
    }

    /// Takes a rating in one run, then starts the app again.
    fn restarted(tag: &str, before: Rating, history: &[GameKey]) -> (PathBuf, GameMemory) {
        let dir = temp_dir(tag);
        let gm = GameMemory::default();
        gm.load(&dir).unwrap();
        gm.set_enabled(true).unwrap();
        let reader = FakeReader::new(vec![Ok(lobby())], vec![Ok(before)]);
        in_lobby_after(&gm, &reader, history, Some(true), 2 * RATING_EVERY);
        assert_eq!(gm.view().rating.duos, before.duos);
        assert!(dir.join(RATING_FILE).exists());
        let again = GameMemory::default();
        again.load(&dir).unwrap();
        (dir, again)
    }

    #[test]
    fn the_change_is_known_across_a_restart_of_the_app() {
        // The case reported on 2026-10-11: Duos 7197, the app restarts, one
        // Duos game, 7272 on the lobby screen.
        let history = [key(1), key(2)];
        let (_, gm) = restarted("rating-restart", rating(6651, 7197), &history[..1]);
        assert_eq!(gm.view().rating.duos, None, "a saved rating is not shown");
        let reader = FakeReader::new(vec![Ok(lobby())], vec![Ok(rating(6651, 7272))]);
        in_lobby_after(&gm, &reader, &history, Some(true), 2 * RATING_EVERY);
        assert_eq!(gm.view().rating.duos, Some(7272));
        assert_eq!(
            gm.view().rating_change,
            Some(RatingChange {
                duos: true,
                delta: 75
            })
        );
    }

    #[test]
    fn a_restart_with_no_game_in_between_keeps_the_base() {
        let history = [key(1), key(2)];
        let (_, gm) = restarted("rating-restart-same", rating(6651, 7197), &history[..1]);
        // Same history, same value: the base stays settled for the next game.
        let reader = FakeReader::new(
            vec![Ok(lobby())],
            vec![
                Ok(rating(6651, 7197)),
                Ok(rating(6651, 7197)),
                Ok(rating(6651, 7272)),
            ],
        );
        in_lobby_after(&gm, &reader, &history[..1], Some(true), 2 * RATING_EVERY);
        assert_eq!(gm.view().rating_change, None);
        leave(&gm, &reader);
        in_lobby_after(&gm, &reader, &history, Some(true), 2 * RATING_EVERY);
        assert_eq!(gm.view().rating_change.map(|c| c.delta), Some(75));
    }

    #[test]
    fn a_saved_rating_two_games_old_or_from_another_build_gives_no_change() {
        let history = [key(1), key(2), key(3)];
        let (_, gm) = restarted("rating-restart-gap", rating(6651, 7197), &history[..1]);
        let reader = FakeReader::new(vec![Ok(lobby())], vec![Ok(rating(6651, 7272))]);
        in_lobby_after(&gm, &reader, &history, Some(true), 2 * RATING_EVERY);
        assert_eq!(gm.view().rating.duos, Some(7272));
        assert_eq!(gm.view().rating_change, None, "two games since");

        let (dir, _) = restarted("rating-restart-build", rating(6651, 7197), &history[..2]);
        let text = std::fs::read_to_string(dir.join(RATING_FILE)).unwrap();
        std::fs::write(dir.join(RATING_FILE), text.replace("253216", "253000")).unwrap();
        let gm = GameMemory::default();
        gm.load(&dir).unwrap();
        in_lobby_after(&gm, &reader, &history, Some(true), 2 * RATING_EVERY);
        assert_eq!(gm.view().rating_change, None, "another build");
    }

    #[test]
    fn the_game_closing_keeps_the_saved_base_for_after_a_restart() {
        let history = [key(1), key(2)];
        let (dir, gm) = restarted("rating-closed", rating(6651, 7197), &history[..1]);
        drop(gm);
        // The app runs on while the game is closed: the build is unknown.
        let gm = GameMemory::default();
        gm.load(&dir).unwrap();
        let reader = FakeReader::new(vec![Ok(lobby())], vec![Ok(rating(6651, 7197))]);
        in_lobby_after(&gm, &reader, &history[..1], Some(true), 2 * RATING_EVERY);
        gm.tick(&reader, &Poll::default());
        assert!(
            dir.join(RATING_FILE).exists(),
            "an unknown build deletes nothing"
        );
        let again = GameMemory::default();
        again.load(&dir).unwrap();
        let reader = FakeReader::new(vec![Ok(lobby())], vec![Ok(rating(6651, 7272))]);
        in_lobby_after(&again, &reader, &history, Some(true), 2 * RATING_EVERY);
        assert_eq!(again.view().rating_change.map(|c| c.delta), Some(75));
    }

    #[test]
    fn a_broken_rating_file_is_ignored_and_off_deletes_it() {
        let history = [key(1), key(2)];
        let (dir, _) = restarted("rating-file", rating(6651, 7197), &history[..1]);
        std::fs::write(dir.join(RATING_FILE), "{ not json").unwrap();
        let gm = GameMemory::default();
        gm.load(&dir).unwrap();
        let reader = FakeReader::new(vec![Ok(lobby())], vec![Ok(rating(6651, 7272))]);
        in_lobby_after(&gm, &reader, &history, Some(true), 2 * RATING_EVERY);
        assert_eq!(gm.view().rating_change, None);
        assert!(dir.join(RATING_FILE).exists(), "the new rating is saved");
        gm.set_enabled(false).unwrap();
        assert!(!dir.join(RATING_FILE).exists());
        let text = r#"{"version":1,"build":253216,"solo":99999,"duos":7197,"after":{"session":"Hearthstone_2026_10_11_01_10_48","index":1},"settled":true}"#;
        std::fs::write(dir.join(RATING_FILE), text).unwrap();
        gm.set_enabled(true).unwrap();
        let gm = GameMemory::default();
        gm.load(&dir).unwrap();
        in_lobby_after(&gm, &reader, &history[..1], Some(true), 2 * RATING_EVERY);
        assert_eq!(gm.view().rating_change, None, "out of range: not a base");
    }

    #[test]
    fn two_rating_reads_that_differ_are_not_taken() {
        let gm = on("rating-differ");
        let reader = FakeReader::new(
            vec![Ok(lobby())],
            vec![
                Ok(rating(6000, 5000)),
                Ok(rating(6100, 5000)),
                Ok(rating(6200, 5000)),
            ],
        );
        in_lobby(&gm, &reader, 0, None, 3 * RATING_EVERY);
        assert_eq!(gm.view().rating.solo, None);
        assert_eq!(gm.view().rating.status, ReaderStatus::Waiting);
    }

    #[test]
    fn the_rating_is_not_read_on_an_unchecked_or_unknown_client_build() {
        let gm = on("rating-build");
        let reader = FakeReader::new(vec![Ok(lobby())], vec![Ok(RATING)]);
        let at = |client_build| Poll {
            scene: Some(Scene::BattlegroundsLobby),
            client_build,
            ..Poll::default()
        };
        for _ in 0..30 {
            gm.tick(&reader, &at(None));
        }
        assert_eq!(reader.calls.get(), 0);
        assert_eq!(gm.view().rating.status, ReaderStatus::Waiting);
        for _ in 0..30 {
            gm.tick(&reader, &at(Some(999_999)));
        }
        assert_eq!(reader.calls.get(), 0);
        assert_eq!(gm.view().rating.status, ReaderStatus::UnsupportedVersion);
        assert_eq!(gm.view().rating.solo, None);
    }

    #[test]
    fn the_change_is_the_last_games_mode_after_exactly_one_game() {
        let gm = on("rating-change");
        let reader = FakeReader::new(
            vec![Ok(lobby())],
            vec![
                Ok(rating(6000, 5000)),
                Ok(rating(6000, 5000)),
                // After one Duos game: the game still shows the old value
                // for a moment, then the new one.
                Ok(rating(6000, 5000)),
                Ok(rating(6000, 5000)),
                Ok(rating(6000, 5040)),
                Ok(rating(6000, 5040)),
            ],
        );
        in_lobby(&gm, &reader, 3, Some(false), 2 * RATING_EVERY);
        assert_eq!(gm.view().rating.duos, Some(5000));
        assert_eq!(
            gm.view().rating_change,
            None,
            "no game since the app started"
        );
        leave(&gm, &reader);
        in_lobby(&gm, &reader, 4, Some(true), 2 * RATING_EVERY);
        assert_eq!(
            gm.view().rating_change,
            None,
            "not updated yet is not a change"
        );
        in_lobby(&gm, &reader, 4, Some(true), 2 * RATING_EVERY);
        assert_eq!(gm.view().rating.duos, Some(5040));
        assert_eq!(
            gm.view().rating_change,
            Some(RatingChange {
                duos: true,
                delta: 40
            })
        );
        // Back to the lobby with no game in between: the change stays.
        leave(&gm, &reader);
        in_lobby(&gm, &reader, 4, Some(true), 2 * RATING_EVERY);
        assert_eq!(gm.view().rating_change.map(|c| c.delta), Some(40));
    }

    #[test]
    fn no_change_is_shown_when_more_than_one_game_was_played_between() {
        let gm = on("rating-gap");
        let reader = FakeReader::new(
            vec![Ok(lobby())],
            vec![
                Ok(rating(6000, 5000)),
                Ok(rating(6000, 5000)),
                Ok(rating(6100, 5000)),
            ],
        );
        in_lobby(&gm, &reader, 3, Some(false), 2 * RATING_EVERY);
        leave(&gm, &reader);
        in_lobby(&gm, &reader, 5, Some(false), 2 * RATING_EVERY);
        assert_eq!(gm.view().rating.solo, Some(6100));
        assert_eq!(gm.view().rating_change, None);
    }

    #[test]
    fn a_rating_taken_before_the_game_updated_it_is_no_base_for_the_next_change() {
        // Game A: the lobby only ever showed the old 6000 (not updated yet).
        // Game B: 6100. The +100 covers two games: no change is shown.
        let gm = on("rating-unsettled");
        let reader = FakeReader::new(
            vec![Ok(lobby())],
            vec![
                Ok(rating(6000, 5000)),
                Ok(rating(6000, 5000)),
                Ok(rating(6000, 5000)),
                Ok(rating(6000, 5000)),
                Ok(rating(6100, 5000)),
            ],
        );
        in_lobby(&gm, &reader, 0, Some(false), 2 * RATING_EVERY);
        leave(&gm, &reader);
        in_lobby(&gm, &reader, 1, Some(false), 2 * RATING_EVERY);
        assert_eq!(gm.view().rating_change, None);
        leave(&gm, &reader);
        in_lobby(&gm, &reader, 2, Some(false), 2 * RATING_EVERY);
        assert_eq!(gm.view().rating.solo, Some(6100));
        assert_eq!(gm.view().rating_change, None);
    }

    #[test]
    fn the_change_goes_away_once_another_game_finishes() {
        let gm = on("rating-stale-change");
        let reader = FakeReader::new(
            vec![Ok(lobby())],
            vec![
                Ok(rating(6000, 5000)),
                Ok(rating(6000, 5000)),
                Ok(rating(6040, 5000)),
                Ok(rating(6040, 5000)),
                Err(ReadFailure::NotYet),
            ],
        );
        in_lobby(&gm, &reader, 0, Some(false), 2 * RATING_EVERY);
        leave(&gm, &reader);
        in_lobby(&gm, &reader, 1, Some(false), 2 * RATING_EVERY);
        assert_eq!(gm.view().rating_change.map(|c| c.delta), Some(40));
        // Game B ends; the lobby cannot be read this time.
        gm.tick(
            &reader,
            &Poll {
                scene: Some(Scene::Game),
                client_build: BUILD,
                finished: 2,
                last_duos: Some(false),
                ..Poll::default()
            },
        );
        assert_eq!(gm.view().rating_change, None);
    }

    #[test]
    fn a_rating_read_on_one_build_is_dropped_when_the_build_changes_or_is_unknown() {
        let gm = on("rating-build-change");
        let reader = FakeReader::new(vec![Ok(lobby())], vec![Ok(RATING)]);
        in_lobby(&gm, &reader, 0, None, 500);
        assert_eq!(gm.view().rating.solo, Some(6543));
        for build in [None, Some(999_999)] {
            gm.tick(
                &reader,
                &Poll {
                    scene: Some(Scene::BattlegroundsLobby),
                    client_build: build,
                    ..Poll::default()
                },
            );
            assert_eq!(gm.view().rating.solo, None, "{build:?}");
            assert_ne!(gm.view().rating.status, ReaderStatus::Ok);
            in_lobby(&gm, &reader, 0, None, 2 * RATING_EVERY);
            leave(&gm, &reader);
        }
    }

    #[test]
    fn a_rating_that_fails_its_checks_is_dropped() {
        let gm = on("rating-bad");
        let reader = FakeReader::new(
            vec![Ok(lobby())],
            vec![
                Ok(RATING),
                Ok(RATING),
                Err(ReadFailure::Unsupported("rating out of range".into())),
            ],
        );
        in_lobby(&gm, &reader, 0, None, 2 * RATING_EVERY);
        assert_eq!(gm.view().rating.solo, Some(6543));
        leave(&gm, &reader);
        in_lobby(&gm, &reader, 0, None, 1);
        let view = gm.view();
        assert_eq!(view.rating.solo, None);
        assert_eq!(view.rating.status, ReaderStatus::UnsupportedVersion);
        let calls = reader.calls.get();
        in_lobby(&gm, &reader, 0, None, 30);
        assert_eq!(reader.calls.get(), calls, "not read again on that visit");
    }

    #[test]
    fn an_unsupported_rating_does_not_mark_good_tribes_unsupported() {
        // A rating the reader cannot use on the lobby screen must not hide
        // that the tribes work.
        let gm = on("rating-unmapped");
        let reader = FakeReader::new(
            vec![Ok(lobby())],
            vec![Err(ReadFailure::Unsupported(
                "class NetCache not found".into(),
            ))],
        );
        ticks(&gm, &reader, Some(&key(1)), Some(Scene::Game), 6);
        ticks(&gm, &reader, None, Some(Scene::BattlegroundsLobby), 3);
        let view = gm.view();
        assert_eq!(view.status, ReaderStatus::Ok);
        assert_eq!(view.tribes_status, ReaderStatus::Ok);
        assert_eq!(view.rating.status, ReaderStatus::UnsupportedVersion);
    }

    #[test]
    fn the_overall_status_follows_the_tribes_and_shows_access_problems() {
        use ReaderStatus::*;
        assert_eq!(overall(Ok, UnsupportedVersion), Ok);
        assert_eq!(overall(UnsupportedVersion, Ok), UnsupportedVersion);
        assert_eq!(overall(Waiting, NotRunning), NotRunning);
        assert_eq!(overall(Waiting, NoAccess), NoAccess);
        assert_eq!(overall(Ok, NoAccess), NoAccess);
        assert_eq!(overall(Waiting, UnsupportedVersion), Waiting);
        assert_eq!(overall(Waiting, Ok), Ok);
    }
}
