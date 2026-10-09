//! Lobby tribes entered by hand (T-303, D-029).
//!
//! The log never says which five tribes are in the lobby, so the user can
//! pick them from a fixed list. The entry lives in its own file next to the
//! history, never inside a game's report: re-reading a game (T-107) rewrites
//! the report and must not lose what the user typed. Entries are keyed by
//! game, like the history. An entry made when no game is in progress waits
//! (`pending`) and attaches to the next game that starts.
//!
//! Only tribe ids from [`TRIBES`] are kept: no free text, so no names can get
//! in. Always shown with the source `entered by you`, never merged with the
//! tribes inferred from the tavern.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::provenance::Source;
use crate::store::GameKey;

pub const FILE_NAME: &str = "lobby_tribes.json";
const FORMAT_VERSION: u32 = 1;
/// A Battlegrounds lobby has five tribes.
pub const LOBBY_TRIBES: usize = 5;

/// The Battlegrounds tribes, named as the log names them (`CARDRACE`), so an
/// entered tribe and an inferred one can be compared. Neutral and "all
/// tribes" are not lobby tribes. Update the list when the game adds a tribe.
pub const TRIBES: [&str; 11] = [
    "ABERRATION",
    "BEAST",
    "DEMON",
    "DRAGON",
    "ELEMENTAL",
    "MECHANICAL",
    "MURLOC",
    "NAGA",
    "PIRATE",
    "QUILBOAR",
    "UNDEAD",
];

/// Why an entry was refused. The message is shown to the user.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntryError {
    NotFiveTribes,
    UnknownTribe,
    DuplicateTribe,
    /// The game is neither in the history nor the one in progress.
    UnknownGame,
    /// Nothing entered for that game.
    NoEntry,
    /// The game already has an entry; clear it first.
    Occupied,
    /// The change could not be saved (the error kind); nothing changed.
    Save(String),
}

impl std::fmt::Display for EntryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntryError::NotFiveTribes => write!(f, "Pick exactly {LOBBY_TRIBES} tribes."),
            EntryError::UnknownTribe => write!(f, "That is not a Battlegrounds tribe."),
            EntryError::DuplicateTribe => write!(f, "Each tribe can be picked once."),
            EntryError::UnknownGame => write!(f, "That game is not in the history."),
            EntryError::NoEntry => write!(f, "No tribes were entered for that game."),
            EntryError::Occupied => {
                write!(f, "That game already has tribes entered. Clear them first.")
            }
            EntryError::Save(kind) => write!(f, "Could not save the tribes ({kind})."),
        }
    }
}

/// Checks five distinct known tribes and returns them in list order, so the
/// same pick always looks the same.
pub fn validate(tribes: &[String]) -> Result<Vec<String>, EntryError> {
    if tribes.len() != LOBBY_TRIBES {
        return Err(EntryError::NotFiveTribes);
    }
    if tribes.iter().any(|t| !TRIBES.contains(&t.as_str())) {
        return Err(EntryError::UnknownTribe);
    }
    let sorted: Vec<String> = TRIBES
        .iter()
        .filter(|known| tribes.iter().any(|t| t == **known))
        .map(|known| known.to_string())
        .collect();
    if sorted.len() != LOBBY_TRIBES {
        return Err(EntryError::DuplicateTribe);
    }
    Ok(sorted)
}

type Games = BTreeMap<GameKey, Vec<String>>;

#[derive(Debug)]
pub struct Entries {
    path: PathBuf,
    games: Games,
    pending: Option<Vec<String>>,
}

impl Entries {
    /// Loads the file; a missing file is an empty set. A file that cannot be
    /// read is an error (and is left alone), never silently replaced.
    pub fn open(dir: &Path) -> io::Result<Entries> {
        fs::create_dir_all(dir)?;
        let path = dir.join(FILE_NAME);
        let mut entries = Entries {
            path,
            games: BTreeMap::new(),
            pending: None,
        };
        let text = match fs::read_to_string(&entries.path) {
            Ok(text) => text,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(entries),
            Err(e) => return Err(e),
        };
        let invalid = |what: &str| io::Error::new(io::ErrorKind::InvalidData, what.to_string());
        let file: FileFormat =
            serde_json::from_str(&text).map_err(|_| invalid("unreadable tribes file"))?;
        if file.version != FORMAT_VERSION {
            return Err(invalid("unknown tribes file version"));
        }
        for game in file.games {
            let tribes = validate(&game.tribes).map_err(|_| invalid("bad tribes entry"))?;
            let key = GameKey {
                session: game.session,
                index: game.index,
            };
            entries.games.insert(key, tribes);
        }
        entries.pending = file
            .pending
            .map(|p| validate(&p).map_err(|_| invalid("bad waiting tribes entry")))
            .transpose()?;
        Ok(entries)
    }

    pub fn entry(&self, key: &GameKey) -> Option<&[String]> {
        self.games.get(key).map(Vec::as_slice)
    }

    pub fn pending(&self) -> Option<&[String]> {
        self.pending.as_deref()
    }

    /// Every game with an entry, oldest first.
    pub fn games(&self) -> impl Iterator<Item = (&GameKey, &[String])> {
        self.games.iter().map(|(k, v)| (k, v.as_slice()))
    }

    /// Enters (or replaces) the tribes of a game.
    pub fn set(&mut self, key: GameKey, tribes: &[String]) -> Result<(), EntryError> {
        let tribes = validate(tribes)?;
        let mut games = self.games.clone();
        games.insert(key, tribes);
        self.commit(games, self.pending.clone())
    }

    pub fn clear(&mut self, key: &GameKey) -> Result<(), EntryError> {
        let mut games = self.games.clone();
        games.remove(key).ok_or(EntryError::NoEntry)?;
        self.commit(games, self.pending.clone())
    }

    /// Moves an entry to another game that has none.
    pub fn move_entry(&mut self, from: &GameKey, to: GameKey) -> Result<(), EntryError> {
        let mut games = self.games.clone();
        let tribes = games.remove(from).ok_or(EntryError::NoEntry)?;
        if *from == to {
            return Ok(());
        }
        if games.contains_key(&to) {
            return Err(EntryError::Occupied);
        }
        games.insert(to, tribes);
        self.commit(games, self.pending.clone())
    }

    pub fn set_pending(&mut self, tribes: &[String]) -> Result<(), EntryError> {
        let tribes = validate(tribes)?;
        self.commit(self.games.clone(), Some(tribes))
    }

    pub fn clear_pending(&mut self) -> Result<(), EntryError> {
        self.commit(self.games.clone(), None)
    }

    /// Attaches the waiting entry to the game that just started, unless that
    /// game already has one. Returns whether it attached.
    pub fn attach_pending(&mut self, started: &GameKey) -> Result<bool, EntryError> {
        let Some(tribes) = self.pending.clone() else {
            return Ok(false);
        };
        if self.games.contains_key(started) {
            return Ok(false);
        }
        let mut games = self.games.clone();
        games.insert(started.clone(), tribes);
        self.commit(games, None).map(|()| true)
    }

    /// Writes the new state to disk and only then takes it, so a failed
    /// write changes nothing. The file is replaced whole (temp file, rename).
    fn commit(&mut self, games: Games, pending: Option<Vec<String>>) -> Result<(), EntryError> {
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
            pending: pending.clone(),
        };
        let save = |e: io::Error| EntryError::Save(format!("{:?}", e.kind()));
        let text = serde_json::to_string_pretty(&file).map_err(|e| save(io::Error::other(e)))?;
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, text).map_err(save)?;
        fs::rename(&tmp, &self.path).map_err(save)?;
        self.games = games;
        self.pending = pending;
        Ok(())
    }
}

/// What the window can ask for. Games are named by session and number, the
/// same as the history rows.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Action {
    /// Tribes of one game (in the history or in progress).
    Set {
        session: String,
        index: u64,
        tribes: Vec<String>,
    },
    Clear {
        session: String,
        index: u64,
    },
    /// Moves an entry to another game that has none.
    Move {
        session: String,
        index: u64,
        to_session: String,
        to_index: u64,
    },
    /// Tribes for the next game, when none is in progress.
    SetPending {
        tribes: Vec<String>,
    },
    ClearPending,
}

/// Runs one action. `is_known` says whether a game is in the history or in
/// progress; entries can only be made for such games.
pub fn apply(
    entries: &mut Entries,
    action: Action,
    is_known: impl Fn(&GameKey) -> bool,
) -> Result<(), EntryError> {
    let game = |session: String, index: u64| -> Result<GameKey, EntryError> {
        let key = GameKey { session, index };
        if is_known(&key) {
            Ok(key)
        } else {
            Err(EntryError::UnknownGame)
        }
    };
    match action {
        Action::Set {
            session,
            index,
            tribes,
        } => entries.set(game(session, index)?, &tribes),
        // A stale entry (its game left the history) must stay clearable.
        Action::Clear { session, index } => entries.clear(&GameKey { session, index }),
        Action::Move {
            session,
            index,
            to_session,
            to_index,
        } => {
            let to = game(to_session, to_index)?;
            entries.move_entry(&GameKey { session, index }, to)
        }
        Action::SetPending { tribes } => entries.set_pending(&tribes),
        Action::ClearPending => entries.clear_pending(),
    }
}

/// Notices when a new game starts and gives it the waiting entry. A game
/// already in progress when the entry was made does not take it: the entry
/// is for the next one.
#[derive(Debug, Default)]
pub struct GameStarts {
    seen: Option<GameKey>,
}

impl GameStarts {
    /// Call after every poll with the game in progress. Returns the game
    /// that got the waiting entry, if any.
    pub fn observe(
        &mut self,
        entries: &mut Entries,
        in_progress: Option<GameKey>,
    ) -> Result<Option<GameKey>, EntryError> {
        let started = match &in_progress {
            Some(key) if self.seen.as_ref() != Some(key) => Some(key.clone()),
            _ => None,
        };
        // Remember the game in progress; between games nothing is in progress,
        // so the next game always looks new.
        self.seen = in_progress;
        match started {
            Some(key) => Ok(entries.attach_pending(&key)?.then_some(key)),
            None => Ok(None),
        }
    }
}

/// An option of the fixed list.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TribeChoice {
    pub id: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GameEntry {
    pub session: String,
    pub index: u64,
    pub tribes: Vec<String>,
}

/// All the window needs to show and edit the entries.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LobbyView {
    /// False when the entries could not be opened (see `problem`).
    pub available: bool,
    pub problem: Option<String>,
    /// The fixed list to pick from.
    pub choices: Vec<TribeChoice>,
    pub lobby_size: usize,
    pub source: Source,
    /// The game being played now, if any.
    pub in_progress: Option<GameEntryKey>,
    /// Entry of the game in progress, if any.
    pub in_progress_tribes: Option<Vec<String>>,
    /// Waiting for the next game.
    pub pending: Option<Vec<String>>,
    pub games: Vec<GameEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GameEntryKey {
    pub session: String,
    pub index: u64,
}

pub fn view(
    entries: Option<&Entries>,
    problem: Option<String>,
    in_progress: Option<&GameKey>,
) -> LobbyView {
    LobbyView {
        available: entries.is_some(),
        problem,
        choices: TRIBES.iter().map(|&id| TribeChoice { id }).collect(),
        lobby_size: LOBBY_TRIBES,
        source: Source::Entered,
        in_progress: in_progress.map(|k| GameEntryKey {
            session: k.session.clone(),
            index: k.index,
        }),
        in_progress_tribes: entries
            .zip(in_progress)
            .and_then(|(e, k)| e.entry(k))
            .map(<[String]>::to_vec),
        pending: entries.and_then(|e| e.pending()).map(<[String]>::to_vec),
        games: entries
            .into_iter()
            .flat_map(Entries::games)
            .map(|(key, tribes)| GameEntry {
                session: key.session.clone(),
                index: key.index,
                tribes: tribes.to_vec(),
            })
            .collect(),
    }
}

#[derive(Serialize, Deserialize)]
struct FileFormat {
    version: u32,
    games: Vec<FileGame>,
    pending: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize)]
struct FileGame {
    session: String,
    index: u64,
    tribes: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn temp_dir() -> PathBuf {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "tavern-ledger-tribes-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn key(index: u64) -> GameKey {
        GameKey {
            session: "Hearthstone_2026_10_09_10_00_00".into(),
            index,
        }
    }

    fn pick(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    const FIVE: [&str; 5] = ["MURLOC", "BEAST", "UNDEAD", "NAGA", "DRAGON"];
    const FIVE_SORTED: [&str; 5] = ["BEAST", "DRAGON", "MURLOC", "NAGA", "UNDEAD"];
    const OTHER: [&str; 5] = ["BEAST", "DEMON", "DRAGON", "ELEMENTAL", "MECHANICAL"];

    #[test]
    fn five_distinct_known_tribes_are_kept_in_list_order() {
        assert_eq!(validate(&pick(&FIVE)).unwrap(), pick(&FIVE_SORTED));
    }

    #[test]
    fn the_wrong_number_of_tribes_is_refused() {
        assert_eq!(validate(&pick(&FIVE[..4])), Err(EntryError::NotFiveTribes));
        let mut six = pick(&FIVE);
        six.push("PIRATE".into());
        assert_eq!(validate(&six), Err(EntryError::NotFiveTribes));
        assert_eq!(validate(&[]), Err(EntryError::NotFiveTribes));
    }

    #[test]
    fn an_unknown_tribe_is_refused_even_if_it_looks_close() {
        for bad in ["Murloc", "NEUTRAL", "ALL", "", "SOMEONE#1234", "MURLOC "] {
            let mut tribes = pick(&FIVE);
            tribes[0] = bad.into();
            assert_eq!(validate(&tribes), Err(EntryError::UnknownTribe), "{bad:?}");
        }
    }

    #[test]
    fn a_repeated_tribe_is_refused() {
        let mut tribes = pick(&FIVE);
        tribes[4] = "MURLOC".into();
        assert_eq!(validate(&tribes), Err(EntryError::DuplicateTribe));
    }

    #[test]
    fn an_entry_is_saved_and_loaded_again() {
        let dir = temp_dir();
        let mut entries = Entries::open(&dir).unwrap();
        assert!(entries.entry(&key(1)).is_none());
        entries.set(key(1), &pick(&FIVE)).unwrap();
        entries.set_pending(&pick(&OTHER)).unwrap();

        let again = Entries::open(&dir).unwrap();
        assert_eq!(again.entry(&key(1)).unwrap(), pick(&FIVE_SORTED));
        assert_eq!(again.pending().unwrap(), pick(&OTHER));
        assert!(again.entry(&key(2)).is_none());
    }

    #[test]
    fn a_refused_entry_changes_nothing() {
        let dir = temp_dir();
        let mut entries = Entries::open(&dir).unwrap();
        entries.set(key(1), &pick(&FIVE)).unwrap();
        let before = entries.entry(&key(1)).unwrap().to_vec();
        assert_eq!(
            entries.set(key(1), &pick(&FIVE[..3])),
            Err(EntryError::NotFiveTribes)
        );
        assert_eq!(entries.entry(&key(1)).unwrap(), before);
        assert_eq!(
            entries.set_pending(&pick(&FIVE[..3])),
            Err(EntryError::NotFiveTribes)
        );
        assert!(entries.pending().is_none());
    }

    #[test]
    fn an_entry_can_be_replaced_and_cleared() {
        let dir = temp_dir();
        let mut entries = Entries::open(&dir).unwrap();
        entries.set(key(1), &pick(&FIVE)).unwrap();
        entries.set(key(1), &pick(&OTHER)).unwrap();
        assert_eq!(entries.entry(&key(1)).unwrap(), pick(&OTHER));
        entries.clear(&key(1)).unwrap();
        assert!(entries.entry(&key(1)).is_none());
        assert_eq!(entries.clear(&key(1)), Err(EntryError::NoEntry));
        assert!(Entries::open(&dir).unwrap().entry(&key(1)).is_none());
    }

    #[test]
    fn a_waiting_entry_attaches_to_the_game_that_starts_and_is_used_up() {
        let dir = temp_dir();
        let mut entries = Entries::open(&dir).unwrap();
        assert!(!entries.attach_pending(&key(1)).unwrap(), "nothing waits");
        entries.set_pending(&pick(&FIVE)).unwrap();
        assert!(entries.attach_pending(&key(2)).unwrap());
        assert!(entries.pending().is_none());
        assert!(entries.entry(&key(2)).is_some());
        assert!(entries.entry(&key(1)).is_none());
        assert!(!entries.attach_pending(&key(3)).unwrap());
        assert!(entries.entry(&key(3)).is_none());
        let again = Entries::open(&dir).unwrap();
        assert!(again.pending().is_none());
        assert!(again.entry(&key(2)).is_some());
    }

    #[test]
    fn a_waiting_entry_never_overwrites_the_entry_of_the_game_that_starts() {
        let dir = temp_dir();
        let mut entries = Entries::open(&dir).unwrap();
        entries.set(key(2), &pick(&FIVE)).unwrap();
        entries.set_pending(&pick(&OTHER)).unwrap();
        assert!(!entries.attach_pending(&key(2)).unwrap());
        assert_eq!(entries.entry(&key(2)).unwrap(), pick(&FIVE_SORTED));
        assert!(entries.pending().is_some(), "still waiting");
    }

    #[test]
    fn an_entry_moves_to_a_game_without_one_and_never_over_another() {
        let dir = temp_dir();
        let mut entries = Entries::open(&dir).unwrap();
        entries.set(key(1), &pick(&FIVE)).unwrap();
        entries.move_entry(&key(1), key(2)).unwrap();
        assert!(entries.entry(&key(1)).is_none());
        assert!(entries.entry(&key(2)).is_some());
        assert_eq!(
            entries.move_entry(&key(1), key(3)),
            Err(EntryError::NoEntry)
        );
        entries.set(key(3), &pick(&FIVE)).unwrap();
        assert_eq!(
            entries.move_entry(&key(2), key(3)),
            Err(EntryError::Occupied)
        );
        assert!(
            entries.entry(&key(2)).is_some(),
            "the refused move changed nothing"
        );
        let again = Entries::open(&dir).unwrap();
        assert!(again.entry(&key(2)).is_some() && again.entry(&key(1)).is_none());
    }

    fn known(keys: &[u64]) -> impl Fn(&GameKey) -> bool + '_ {
        move |k: &GameKey| keys.contains(&k.index)
    }

    fn set(index: u64, list: &[&str]) -> Action {
        Action::Set {
            session: key(index).session,
            index,
            tribes: pick(list),
        }
    }

    #[test]
    fn actions_only_touch_games_that_are_known() {
        let dir = temp_dir();
        let mut entries = Entries::open(&dir).unwrap();
        apply(&mut entries, set(1, &FIVE), known(&[1])).unwrap();
        assert!(entries.entry(&key(1)).is_some());
        assert_eq!(
            apply(&mut entries, set(9, &FIVE), known(&[1])),
            Err(EntryError::UnknownGame)
        );
        assert!(entries.entry(&key(9)).is_none());
        // Wrong input is refused by the same path.
        assert_eq!(
            apply(&mut entries, set(1, &FIVE[..2]), known(&[1])),
            Err(EntryError::NotFiveTribes)
        );
    }

    #[test]
    fn a_stale_entry_can_be_cleared_even_when_its_game_is_gone() {
        let dir = temp_dir();
        let mut entries = Entries::open(&dir).unwrap();
        entries.set(key(7), &pick(&FIVE)).unwrap();
        let clear = Action::Clear {
            session: key(7).session,
            index: 7,
        };
        apply(&mut entries, clear, known(&[])).unwrap();
        assert!(entries.entry(&key(7)).is_none());
    }

    #[test]
    fn the_move_action_needs_a_known_target() {
        let dir = temp_dir();
        let mut entries = Entries::open(&dir).unwrap();
        entries.set(key(1), &pick(&FIVE)).unwrap();
        let to = |index| Action::Move {
            session: key(1).session,
            index: 1,
            to_session: key(index).session,
            to_index: index,
        };
        assert_eq!(
            apply(&mut entries, to(5), known(&[1, 2])),
            Err(EntryError::UnknownGame)
        );
        apply(&mut entries, to(2), known(&[1, 2])).unwrap();
        assert!(entries.entry(&key(1)).is_none() && entries.entry(&key(2)).is_some());
    }

    #[test]
    fn the_waiting_entry_is_set_and_cleared_by_actions() {
        let dir = temp_dir();
        let mut entries = Entries::open(&dir).unwrap();
        let tribes = pick(&FIVE);
        apply(&mut entries, Action::SetPending { tribes }, known(&[])).unwrap();
        assert!(entries.pending().is_some());
        apply(&mut entries, Action::ClearPending, known(&[])).unwrap();
        assert!(entries.pending().is_none());
    }

    #[test]
    fn the_waiting_entry_goes_to_a_game_that_starts_after_it_was_made() {
        let dir = temp_dir();
        let mut entries = Entries::open(&dir).unwrap();
        let mut starts = GameStarts::default();
        assert_eq!(starts.observe(&mut entries, None).unwrap(), None);
        entries.set_pending(&pick(&FIVE)).unwrap();
        assert_eq!(starts.observe(&mut entries, None).unwrap(), None);
        assert_eq!(
            starts.observe(&mut entries, Some(key(2))).unwrap(),
            Some(key(2))
        );
        assert!(entries.entry(&key(2)).is_some() && entries.pending().is_none());
        // The same game seen again changes nothing.
        assert_eq!(starts.observe(&mut entries, Some(key(2))).unwrap(), None);
    }

    #[test]
    fn a_game_already_in_progress_does_not_take_an_entry_made_for_the_next_one() {
        let dir = temp_dir();
        let mut entries = Entries::open(&dir).unwrap();
        let mut starts = GameStarts::default();
        starts.observe(&mut entries, Some(key(1))).unwrap();
        entries.set_pending(&pick(&FIVE)).unwrap();
        assert_eq!(starts.observe(&mut entries, Some(key(1))).unwrap(), None);
        assert!(entries.entry(&key(1)).is_none() && entries.pending().is_some());
        // The game ends, the next one starts and takes it.
        starts.observe(&mut entries, None).unwrap();
        assert_eq!(
            starts.observe(&mut entries, Some(key(2))).unwrap(),
            Some(key(2))
        );
    }

    #[test]
    fn the_view_says_what_is_entered_and_where_it_comes_from() {
        let dir = temp_dir();
        let mut entries = Entries::open(&dir).unwrap();
        entries.set(key(1), &pick(&FIVE)).unwrap();
        entries.set(key(3), &pick(&OTHER)).unwrap();
        entries.set_pending(&pick(&OTHER)).unwrap();
        let v = view(Some(&entries), None, Some(&key(3)));
        assert!(v.available && v.problem.is_none());
        assert_eq!(v.source, Source::Entered);
        assert_eq!(v.lobby_size, 5);
        assert_eq!(v.choices.len(), TRIBES.len());
        assert_eq!(v.in_progress.unwrap().index, 3);
        assert_eq!(v.in_progress_tribes.unwrap(), pick(&OTHER));
        assert_eq!(v.pending.unwrap(), pick(&OTHER));
        let numbers: Vec<_> = v.games.iter().map(|g| g.index).collect();
        assert_eq!(numbers, [1, 3]);
    }

    #[test]
    fn the_view_without_entries_says_why_and_still_lists_the_choices() {
        let v = view(None, Some("busy".into()), None);
        assert!(!v.available);
        assert_eq!(v.problem.as_deref(), Some("busy"));
        assert_eq!(v.choices.len(), TRIBES.len());
        assert!(v.games.is_empty() && v.pending.is_none() && v.in_progress_tribes.is_none());
    }

    #[test]
    fn a_file_that_cannot_be_read_is_an_error_and_is_left_alone() {
        let dir = temp_dir();
        let path = dir.join(FILE_NAME);
        fs::write(&path, "{ not json").unwrap();
        assert!(Entries::open(&dir).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "{ not json");
        fs::write(&path, r#"{"version":99,"games":[],"pending":null}"#).unwrap();
        assert!(Entries::open(&dir).is_err(), "unknown version");
    }

    #[test]
    fn a_file_with_a_bad_entry_is_an_error_not_a_guess() {
        let dir = temp_dir();
        let body = r#"{"version":1,"games":[{"session":"S","index":1,"tribes":["MURLOC"]}],"pending":null}"#;
        fs::write(dir.join(FILE_NAME), body).unwrap();
        assert!(Entries::open(&dir).is_err());
    }

    #[test]
    fn the_file_holds_tribe_ids_and_game_keys_only() {
        let dir = temp_dir();
        let mut entries = Entries::open(&dir).unwrap();
        entries.set(key(1), &pick(&FIVE)).unwrap();
        let text = fs::read_to_string(dir.join(FILE_NAME)).unwrap();
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        let game = &value["games"][0];
        let mut fields: Vec<_> = game.as_object().unwrap().keys().cloned().collect();
        fields.sort();
        assert_eq!(fields, ["index", "session", "tribes"]);
    }
}
