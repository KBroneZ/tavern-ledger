//! What the game has shown so far, against what its log already says (T-335).
//!
//! The game writes each packet from the server twice to `Power.log`: once as
//! it arrives (`GameState.DebugPrintPower`, the lines the parser reads) and
//! once as the client plays it on screen (`PowerTaskList.DebugPrintPower`).
//! A Battlegrounds combat arrives whole when it starts, so the log has its
//! outcome (health, leaderboard places, the next turn) a median of about 20 s
//! and up to 72 s before the game shows it (the user's three sessions of
//! 2026-10-11). The overlay matched the hovered slot with places the screen
//! did not show yet, so it could show another opponent's card, and it showed
//! combat results early (D-004: only what the player has seen).
//!
//! Both kinds of line carry the same `TAG_CHANGE`s in the same order (98,716
//! of each in one session, the same count at every game start and session
//! end). So views of the live state are kept with the number of arrived
//! `TAG_CHANGE`s they were built from, and the one shown is the newest whose
//! number the played lines have reached. Each game start (`CREATE_GAME`,
//! written as both kinds) sets the played count back in step, so a game whose
//! end was never played (a concede, a reconnect) does not hold the next one. A
//! view held longer than [`MAX_HOLD`] is shown anyway, so a log that never
//! catches up cannot freeze the overlay; a log with no played lines at all is
//! not held. Before the game has played any kept view (the app started during
//! a fight), the oldest one kept is shown: the nearest to the screen there is.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Longer than the longest lag measured (72 s).
pub const MAX_HOLD: Duration = Duration::from_secs(120);
/// Views kept at most; the oldest waiting one goes first.
const MAX_VIEWS: usize = 32;

const ARRIVED: &str = "GameState.DebugPrintPower() -";
const PLAYED: &str = "PowerTaskList.DebugPrintPower() -";

/// Is this a line of the given kind whose packet starts with `word`?
fn packet(line: &str, kind: &str, word: &str) -> bool {
    line.split_once(kind)
        .is_some_and(|(_, body)| body.trim_start().starts_with(word))
}

struct Held<T> {
    arrived: u64,
    at: Instant,
    view: T,
}

pub struct Playback<T> {
    arrived: u64,
    played: u64,
    /// The arrived count at each game start not played yet.
    starts: VecDeque<u64>,
    views: VecDeque<Held<T>>,
}

impl<T> Default for Playback<T> {
    fn default() -> Self {
        Playback {
            arrived: 0,
            played: 0,
            starts: VecDeque::new(),
            views: VecDeque::new(),
        }
    }
}

impl<T> Playback<T> {
    /// Counts one line of `Power.log`.
    pub fn feed(&mut self, line: &str) {
        if packet(line, ARRIVED, "TAG_CHANGE") {
            self.arrived += 1;
        } else if packet(line, PLAYED, "TAG_CHANGE") {
            self.played += 1;
        } else if packet(line, ARRIVED, "CREATE_GAME") {
            self.starts.push_back(self.arrived);
        } else if packet(line, PLAYED, "CREATE_GAME") {
            if let Some(at) = self.starts.pop_front() {
                self.played = at;
            }
        }
    }

    fn shown(&self, held: &Held<T>, now: Instant) -> bool {
        self.played == 0
            || self.played >= held.arrived
            || now.saturating_duration_since(held.at) >= MAX_HOLD
    }

    /// Keeps `view`, the live state built from every line read so far.
    pub fn record(&mut self, view: T, now: Instant) {
        if self.views.len() >= MAX_VIEWS {
            // Keep the one on screen; drop the oldest one still waiting.
            self.views.remove(1);
        }
        self.views.push_back(Held {
            arrived: self.arrived,
            at: now,
            view,
        });
        // Views older than the newest shown one are never shown again.
        while self.views.len() > 1 && self.shown(&self.views[1], now) {
            self.views.pop_front();
        }
    }

    /// The newest view the game has shown, else the oldest one kept;
    /// `None` only before any view is kept.
    pub fn view(&self, now: Instant) -> Option<&T> {
        self.views
            .iter()
            .rev()
            .find(|held| self.shown(held, now))
            .or_else(|| self.views.front())
            .map(|held| &held.view)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GS: &str = "D 10:00:00.0000000 GameState.DebugPrintPower() -     TAG_CHANGE Entity=GameEntity tag=TURN value=2";
    const PT: &str = "D 10:00:00.0000000 PowerTaskList.DebugPrintPower() -     TAG_CHANGE Entity=GameEntity tag=TURN value=2";
    const OTHER: &str =
        "D 10:00:00.0000000 GameState.DebugPrintPower() - BLOCK_START BlockType=TRIGGER";

    fn feed(p: &mut Playback<&'static str>, line: &str, times: usize) {
        (0..times).for_each(|_| p.feed(line));
    }

    #[test]
    fn a_view_waits_until_the_game_has_played_its_lines() {
        let t = Instant::now();
        let mut p = Playback::default();
        feed(&mut p, GS, 10);
        feed(&mut p, PT, 10);
        p.record("before combat", t);
        feed(&mut p, GS, 500);
        feed(&mut p, OTHER, 3);
        p.record("after combat", t);
        assert_eq!(p.view(t), Some(&"before combat"));
        feed(&mut p, PT, 499);
        assert_eq!(p.view(t), Some(&"before combat"), "one line still to play");
        feed(&mut p, PT, 1);
        assert_eq!(p.view(t), Some(&"after combat"));
    }

    #[test]
    fn a_log_without_played_lines_is_not_held() {
        let t = Instant::now();
        let mut p = Playback::default();
        feed(&mut p, GS, 100);
        p.record("now", t);
        assert_eq!(p.view(t), Some(&"now"));
    }

    #[test]
    fn before_any_view_is_played_the_oldest_kept_is_shown() {
        let t = Instant::now();
        let mut p = Playback::default();
        assert_eq!(p.view(t), None, "nothing kept yet");
        feed(&mut p, GS, 50);
        feed(&mut p, PT, 1);
        p.record("first", t);
        feed(&mut p, GS, 50);
        p.record("second", t);
        assert_eq!(p.view(t), Some(&"first"));
        feed(&mut p, PT, 49);
        assert_eq!(p.view(t), Some(&"first"));
        feed(&mut p, PT, 50);
        assert_eq!(p.view(t), Some(&"second"));
    }

    #[test]
    fn the_first_played_line_never_hides_a_view_shown_before() {
        let t = Instant::now();
        let mut p = Playback::default();
        feed(&mut p, GS, 20);
        p.record("shown", t);
        assert_eq!(p.view(t), Some(&"shown"));
        feed(&mut p, PT, 1);
        assert_eq!(p.view(t), Some(&"shown"));
    }

    const GS_START: &str = "D 10:00:00.0000000 GameState.DebugPrintPower() - CREATE_GAME";
    const PT_START: &str = "D 10:00:00.0000000 PowerTaskList.DebugPrintPower() -     CREATE_GAME";

    #[test]
    fn a_game_whose_end_was_never_played_does_not_hold_the_next() {
        let t = Instant::now();
        let mut p = Playback::default();
        p.feed(GS_START);
        p.feed(PT_START);
        feed(&mut p, GS, 100);
        feed(&mut p, PT, 90); // the last fight was never played (a concede)
        p.feed(GS_START);
        feed(&mut p, GS, 5);
        p.record("old game", t);
        p.feed(PT_START);
        feed(&mut p, PT, 4);
        feed(&mut p, GS, 1);
        p.record("new game", t);
        feed(&mut p, PT, 1);
        assert_eq!(p.view(t), Some(&"old game"), "back in step: 105 of 106");
        feed(&mut p, PT, 1);
        assert_eq!(p.view(t), Some(&"new game"));
    }

    #[test]
    fn a_view_held_too_long_is_shown_anyway() {
        let t = Instant::now();
        let mut p = Playback::default();
        feed(&mut p, GS, 10);
        feed(&mut p, PT, 10);
        p.record("old", t);
        feed(&mut p, GS, 10);
        p.record("stuck", t + Duration::from_secs(1));
        assert_eq!(p.view(t + Duration::from_secs(60)), Some(&"old"));
        assert_eq!(
            p.view(t + MAX_HOLD + Duration::from_secs(1)),
            Some(&"stuck")
        );
    }

    #[test]
    fn views_are_trimmed_and_capped() {
        let t = Instant::now();
        let mut p = Playback::default();
        feed(&mut p, GS, 1);
        feed(&mut p, PT, 1);
        p.record("shown", t);
        for _ in 0..100 {
            feed(&mut p, GS, 1);
            p.record("waiting", t);
        }
        assert!(p.views.len() <= MAX_VIEWS);
        assert_eq!(p.view(t), Some(&"shown"), "the one on screen is kept");
        feed(&mut p, PT, 100);
        p.record("caught up", t);
        assert_eq!(p.views.len(), 1);
        assert_eq!(p.view(t), Some(&"caught up"));
    }

    #[test]
    fn only_tag_changes_are_counted() {
        let mut p: Playback<()> = Playback::default();
        p.feed(OTHER);
        p.feed("D 10:00:00.0000000 PowerTaskList.DebugDump() - Block Start=(null)");
        p.feed("D 10:00:00.0000000 GameState.DebugPrintPower() -     FULL_ENTITY - Creating ID=5 CardID=");
        assert_eq!((p.arrived, p.played), (0, 0));
        p.feed(GS);
        p.feed(PT);
        assert_eq!((p.arrived, p.played), (1, 1));
    }
}
