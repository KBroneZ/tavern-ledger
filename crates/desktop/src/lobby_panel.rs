//! What the overlay's lobby panel shows (T-338, T-366): on the Battlegrounds
//! lobby screen, before a game, the player's own rating (from the game, when
//! the D-063 switch is on), the day's last games with their hero, place and
//! rating change, and a one-line summary of the day's games per mode. The page
//! renders it and decides nothing.
//!
//! The app has no leaderboard rating of its own yet (the public leaderboard
//! client is a prototype, `tools/leaderboard.py`): with the switch off, no
//! rating is shown, never a guess.

use serde::Serialize;
use tracker::game_clock::{session_start, LocalToUtc};
use tracker::game_values::OwnRating;
use tracker::lobby_recap::SessionRecap;
use tracker::stats::ModeSummary;

/// A day is at most 25 hours long (a clock change); anything else is not a
/// day the page asked for.
const MAX_DAY: i64 = 25 * 3600;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LobbyView {
    /// On the Battlegrounds lobby screen with no game on.
    pub in_lobby: bool,
    /// From the game (D-063); status `off` when the switch is off.
    pub rating: OwnRating,
    /// The day's games, Solo and Duos apart; only modes played.
    pub today: Vec<ModeSummary>,
    /// The day's last games with their hero, place and rating change, and the
    /// rating the day started with (T-366); no rating while the read is off.
    pub recap: SessionRecap,
}

/// Whether the page's day (`start`..`end`, UTC seconds of the viewer's local
/// midnights) is one: positive and at most [`MAX_DAY`] long.
pub fn sane_day(start: i64, end: i64) -> bool {
    end.checked_sub(start)
        .is_some_and(|span| span > 0 && span <= MAX_DAY)
}

/// Whether a game was played in the day: when it ended (UTC, from the log's
/// clock), else when its session started (the folder name, local time).
pub fn played_in(ended: Option<i64>, session: &str, day: (i64, i64), to_utc: LocalToUtc) -> bool {
    let when = ended.or_else(|| session_start(session).and_then(to_utc));
    when.is_some_and(|t| day.0 <= t && t < day.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utc_is_local(local: i64) -> Option<i64> {
        Some(local)
    }

    // 2026-10-11 00:00:00 UTC.
    const DAY: (i64, i64) = (1_791_676_800, 1_791_676_800 + 86_400);

    #[test]
    fn a_game_counts_by_when_it_ended_else_by_its_session_start() {
        let s = "Hearthstone_2026_10_10_23_50_00";
        assert!(played_in(Some(DAY.0 + 600), s, DAY, utc_is_local));
        assert!(!played_in(Some(DAY.0 - 1), s, DAY, utc_is_local));
        assert!(!played_in(Some(DAY.1), s, DAY, utc_is_local));
        assert!(
            !played_in(None, s, DAY, utc_is_local),
            "session began the day before"
        );
        assert!(played_in(
            None,
            "Hearthstone_2026_10_11_04_58_30",
            DAY,
            utc_is_local
        ));
        assert!(!played_in(None, "not a session", DAY, utc_is_local));
    }

    #[test]
    fn only_a_real_day_is_accepted() {
        assert!(sane_day(DAY.0, DAY.1));
        assert!(sane_day(DAY.0, DAY.0 + 23 * 3600));
        assert!(!sane_day(DAY.1, DAY.0));
        assert!(!sane_day(DAY.0, DAY.0 + 30 * 86_400));
        assert!(!sane_day(-(1 << 62), 1 << 62), "no overflow");
    }
}
