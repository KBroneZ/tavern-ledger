//! Values read from the game itself (D-063), when the user turned that on:
//! the lobby's five tribes and the player's own Battlegrounds rating. The
//! reader lives in the `game-memory` crate and runs in the app; this module
//! only decides how its values are used and labelled.
//!
//! - The tribes from the game come first (before the ones entered by hand
//!   and the inferred ones), labelled "from the game", but only while every
//!   tribe the tavern confirms (D-050) is among them. If the tavern
//!   contradicts them, they are dropped for the rest of the game and the app
//!   says "unsupported game version", falling back to the other sources.
//! - On a game build the reader was not checked on (or an unknown build),
//!   nothing from the game is shown: "unsupported game version" (D-063's
//!   fail-safe rule). The app does not even read on such a build.
//! - The values stay on this PC: they are never put in a game's report and
//!   never uploaded.

use serde::Serialize;

use crate::lobby_tribes::{LOBBY_TRIBES, TRIBES};

/// Game builds the reader's paths were checked on, on the user's client.
pub const CHECKED_BUILDS: &[i64] = &[253216];

/// What the overlay shows when the game's value cannot be used.
pub const UNSUPPORTED_TRIBES: &str = "Lobby tribes: unsupported game version";
pub const UNSUPPORTED_RATING: &str = "Rating: unsupported game version";

/// How the reader is doing, for the settings and the overlay.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReaderStatus {
    /// The switch is off (the default): the game is never opened.
    #[default]
    Off,
    /// The last read gave a value that passed every check.
    Ok,
    /// On, but nothing to read yet (no game, or hero select still loading).
    Waiting,
    /// The game's structure or a check did not match; the app falls back to
    /// its log-based values.
    UnsupportedVersion,
    NotRunning,
    /// Windows refused read access (the game runs with more rights than
    /// the app). Never retried with more rights.
    NoAccess,
}

/// The tribes read from the game for the game in progress, with the reader's
/// status and whether the game build is one the reader was checked on.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct GameTribes {
    /// Five tribes, sorted, or empty.
    pub tribes: Vec<String>,
    pub status: ReaderStatus,
    /// False on a build not in [`CHECKED_BUILDS`] (or an unknown build);
    /// then there are no tribes.
    pub checked_build: bool,
    /// The note to show instead, when the value was refused.
    pub note: Option<&'static str>,
}

impl GameTribes {
    /// What the reader gave for this game. `build` is the game's build from
    /// the log.
    pub fn new(tribes: Option<&[String]>, status: ReaderStatus, build: Option<i64>) -> GameTribes {
        let tribes = tribes.and_then(valid).unwrap_or_default();
        let checked_build = build.is_some_and(|b| CHECKED_BUILDS.contains(&b));
        let status = if status == ReaderStatus::Ok && (tribes.is_empty() || !checked_build) {
            ReaderStatus::UnsupportedVersion
        } else {
            status
        };
        GameTribes {
            tribes: if status == ReaderStatus::Ok {
                tribes
            } else {
                Vec::new()
            },
            status,
            checked_build,
            note: (status == ReaderStatus::UnsupportedVersion).then_some(UNSUPPORTED_TRIBES),
        }
    }

    /// Drops the tribes if the tavern confirmed one that is not among them:
    /// the read was wrong, so nothing from it is shown.
    pub fn checked_against(self, confirmed: Option<&[String]>) -> GameTribes {
        let contradicted = confirmed
            .unwrap_or_default()
            .iter()
            .filter(|t| TRIBES.contains(&t.as_str()))
            .any(|t| !self.tribes.contains(t));
        if self.tribes.is_empty() || !contradicted {
            return self;
        }
        GameTribes {
            tribes: Vec::new(),
            status: ReaderStatus::UnsupportedVersion,
            note: Some(UNSUPPORTED_TRIBES),
            ..self
        }
    }
}

/// Five distinct known tribes, sorted; anything else is refused.
fn valid(tribes: &[String]) -> Option<Vec<String>> {
    let mut out: Vec<String> = tribes.to_vec();
    out.sort();
    out.dedup();
    (out.len() == LOBBY_TRIBES
        && tribes.len() == LOBBY_TRIBES
        && out.iter().all(|t| TRIBES.contains(&t.as_str())))
    .then_some(out)
}

/// The player's own rating as the game shows it, kept on this PC.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct OwnRating {
    /// None when the game shows none for that mode, or nothing was read.
    pub solo: Option<u32>,
    pub duos: Option<u32>,
    pub status: ReaderStatus,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    const LOBBY: [&str; 5] = ["PIRATE", "BEAST", "DRAGON", "MURLOC", "UNDEAD"];

    #[test]
    fn five_tribes_from_the_game_are_kept_sorted_on_a_checked_build() {
        let g = GameTribes::new(Some(&strings(&LOBBY)), ReaderStatus::Ok, Some(253216));
        assert_eq!(
            g.tribes,
            strings(&["BEAST", "DRAGON", "MURLOC", "PIRATE", "UNDEAD"])
        );
        assert_eq!(g.status, ReaderStatus::Ok);
        assert!(g.checked_build);
        assert_eq!(g.note, None);
    }

    #[test]
    fn an_unchecked_or_unknown_build_is_never_shown() {
        // Fail safe (D-063): the reader's path is only trusted on the builds
        // it was checked on; any other build says "unsupported game version".
        for build in [Some(999_999), None] {
            let g = GameTribes::new(Some(&strings(&LOBBY)), ReaderStatus::Ok, build);
            assert!(g.tribes.is_empty());
            assert!(!g.checked_build);
            assert_eq!(g.status, ReaderStatus::UnsupportedVersion);
            assert_eq!(g.note, Some(UNSUPPORTED_TRIBES));
        }
    }

    #[test]
    fn a_bad_list_is_never_shown_and_says_unsupported() {
        for bad in [
            strings(&["BEAST", "DRAGON", "MURLOC", "PIRATE"]),
            strings(&["BEAST", "BEAST", "MURLOC", "PIRATE", "UNDEAD"]),
            strings(&["BEAST", "TOTEM", "MURLOC", "PIRATE", "UNDEAD"]),
        ] {
            let g = GameTribes::new(Some(&bad), ReaderStatus::Ok, Some(253216));
            assert!(g.tribes.is_empty());
            assert_eq!(g.status, ReaderStatus::UnsupportedVersion);
            assert_eq!(g.note, Some(UNSUPPORTED_TRIBES));
        }
    }

    #[test]
    fn statuses_other_than_ok_carry_no_tribes() {
        for status in [
            ReaderStatus::Off,
            ReaderStatus::Waiting,
            ReaderStatus::NotRunning,
            ReaderStatus::NoAccess,
        ] {
            let g = GameTribes::new(Some(&strings(&LOBBY)), status, Some(253216));
            assert!(g.tribes.is_empty());
            assert_eq!(g.status, status);
            assert_eq!(g.note, None);
        }
        let g = GameTribes::new(None, ReaderStatus::UnsupportedVersion, None);
        assert_eq!(g.note, Some(UNSUPPORTED_TRIBES));
    }

    #[test]
    fn the_tavern_agreeing_keeps_them_and_a_contradiction_drops_them() {
        let g = GameTribes::new(Some(&strings(&LOBBY)), ReaderStatus::Ok, Some(253216));
        let agree = g
            .clone()
            .checked_against(Some(&strings(&["BEAST", "UNDEAD"])));
        assert_eq!(agree, g);
        let none = g.clone().checked_against(None);
        assert_eq!(none, g);
        let wrong = g.checked_against(Some(&strings(&["BEAST", "DEMON"])));
        assert!(wrong.tribes.is_empty());
        assert_eq!(wrong.status, ReaderStatus::UnsupportedVersion);
        assert_eq!(wrong.note, Some(UNSUPPORTED_TRIBES));
    }
}
