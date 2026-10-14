//! The session recap on the overlay's lobby panel (T-366): the last games of
//! the day as small tiles, each with its hero, its place and the rating it won
//! or lost, and the rating the day started with. The window renders it and
//! decides nothing.
//!
//! The ratings come from [`crate::game_ratings`] (kept only while the read
//! from the game is on, D-063, D-147); without them a tile has a hero and a
//! place and nothing else, never a guessed number. A game with no counted
//! place (not finished in the log) has none either.

use serde::Serialize;
use serde_json::Value;

use crate::game_ratings::SavedRatings;
use crate::recap::{text, HeroRef, View};
use crate::stats::{counted_place, mode_of, DUOS, OTHER};
use crate::store::GameKey;

/// How many games the panel shows.
pub const TILES: usize = 8;

/// One game the app has seen, in the order it was played. Games the stats do
/// not count (played with the reconnect dev tool) are in the list, because
/// they moved the rating, but they are not tiles.
pub struct HistoryGame<'a> {
    pub key: GameKey,
    pub report: &'a Value,
    /// Played in the day the panel is about.
    pub in_day: bool,
    /// Counted by the stats: the games that show as tiles.
    pub counted: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RecapTile {
    /// The hero played; None when the log did not say.
    pub hero: Option<HeroRef>,
    pub duos: bool,
    /// The place when it counts (a finished game).
    pub place: Option<i64>,
    /// The rating won (positive) or lost, in this game's mode; None when the
    /// app does not know it.
    pub change: Option<i32>,
}

/// The ratings the day started with, per mode; None when not known.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Starting {
    pub solo: Option<u32>,
    pub duos: Option<u32>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct SessionRecap {
    /// The day's last games, the oldest first (at most [`TILES`]).
    pub tiles: Vec<RecapTile>,
    /// Games played in the day, Solo and Duos (tiles show the last ones).
    pub games: usize,
    pub starting: Starting,
}

/// The recap of the day from the whole history (`history`, oldest first). `ratings` is None when the read from the game is
/// off or its file is not open.
pub fn session_recap(history: &[HistoryGame], ratings: Option<&SavedRatings>) -> SessionRecap {
    let played: Vec<(usize, &HistoryGame)> = history
        .iter()
        .enumerate()
        .filter(|(_, g)| g.in_day && g.counted && mode_of(g.report) != OTHER)
        .collect();
    let previous = |at: usize| at.checked_sub(1).map(|i| &history[i].key);
    let tiles = played
        .iter()
        .skip(played.len().saturating_sub(TILES))
        .map(|&(at, game)| tile(game, previous(at), ratings))
        .collect();
    let starting = match (played.first(), ratings) {
        (Some(&(at, _)), Some(ratings)) => {
            let (solo, duos) = ratings.starting(previous(at));
            Starting { solo, duos }
        }
        _ => Starting::default(),
    };
    SessionRecap {
        tiles,
        games: played.len(),
        starting,
    }
}

fn tile(
    game: &HistoryGame,
    previous: Option<&GameKey>,
    ratings: Option<&SavedRatings>,
) -> RecapTile {
    let mode = mode_of(game.report);
    let duos = mode == DUOS;
    let status = text(game.report, "status");
    RecapTile {
        hero: text(game.report, "hero").map(|id| View::new(game.report).hero_ref(&id)),
        duos,
        place: counted_place(mode, status.as_deref(), game.report),
        change: ratings.and_then(|r| r.change(&game.key, previous, duos)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_ratings::Taken;
    use crate::stats::SOLO;
    use serde_json::json;
    use std::path::PathBuf;

    fn dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tl-lobby-recap-{tag}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn key(index: u64) -> GameKey {
        GameKey {
            session: "Hearthstone_2026_10_14_20_00_00".into(),
            index,
        }
    }

    fn report(mode: &str, hero: &str, place: i64, status: &str) -> Value {
        json!({
            "status": status, "game_type": mode, "hero": hero, "final_place": place,
            "card_names": {hero: format!("Name of {hero}")},
        })
    }

    /// The reading after game `index` (`None`: before any game), the game
    /// being Duos or not; the game before it is `index - 1`.
    fn take(ratings: &mut SavedRatings, index: Option<u64>, duos: bool, solo: u32, duos_r: u32) {
        let after = index.map(key);
        let previous = index.filter(|i| *i > 1).map(|i| key(i - 1));
        ratings
            .record(Taken {
                after: after.as_ref(),
                previous: previous.as_ref(),
                game_duos: Some(duos),
                first_of_run: index.is_none(),
                solo: Some(solo),
                duos: Some(duos_r),
            })
            .unwrap();
    }

    fn history(reports: &[Value], first_in_day: usize) -> Vec<HistoryGame<'_>> {
        reports
            .iter()
            .enumerate()
            .map(|(i, report)| HistoryGame {
                key: key(i as u64 + 1),
                report,
                in_day: i >= first_in_day,
                counted: true,
            })
            .collect()
    }

    #[test]
    fn tiles_have_the_hero_the_place_and_the_rating_change() {
        let reports = [
            report(SOLO, "H_A", 2, "ok"),
            report(SOLO, "H_B", 6, "ok"),
            report(DUOS, "H_C", 1, "ok"),
        ];
        let mut ratings = SavedRatings::open(&dir("tiles")).unwrap();
        take(&mut ratings, None, false, 6400, 5800);
        take(&mut ratings, Some(1), false, 6440, 5800);
        take(&mut ratings, Some(2), false, 6415, 5800);
        take(&mut ratings, Some(3), true, 6415, 5840);
        let recap = session_recap(&history(&reports, 0), Some(&ratings));
        assert_eq!(recap.games, 3);
        let seen: Vec<_> = recap
            .tiles
            .iter()
            .map(|t| {
                (
                    t.hero.as_ref().unwrap().id.as_str(),
                    t.duos,
                    t.place,
                    t.change,
                )
            })
            .collect();
        assert_eq!(
            seen,
            [
                ("H_A", false, Some(2), Some(40)),
                ("H_B", false, Some(6), Some(-25)),
                ("H_C", true, Some(1), Some(40)),
            ]
        );
        let first = recap.tiles[0].hero.as_ref().unwrap();
        assert_eq!(first.name.as_deref(), Some("Name of H_A"));
        let start = Starting {
            solo: Some(6400),
            duos: Some(5800),
        };
        assert_eq!(recap.starting, start);
    }

    #[test]
    fn without_the_read_there_are_games_and_places_and_no_rating_at_all() {
        let reports = [
            report(SOLO, "H_A", 3, "ok"),
            report(SOLO, "H_B", 5, "incomplete"),
        ];
        let recap = session_recap(&history(&reports, 0), None);
        assert_eq!(recap.games, 2);
        let places: Vec<_> = recap.tiles.iter().map(|t| t.place).collect();
        assert_eq!(places, [Some(3), None]);
        assert!(recap.tiles.iter().all(|t| t.change.is_none()));
        assert_eq!(recap.starting, Starting::default());
    }

    #[test]
    fn only_the_days_last_eight_games_show_and_the_start_is_before_the_first_of_the_day() {
        let reports: Vec<Value> = (1..=11)
            .map(|i| report(SOLO, &format!("H_{i}"), 4, "ok"))
            .collect();
        let mut ratings = SavedRatings::open(&dir("eight")).unwrap();
        // Two games came before the day: the day started after game 2.
        take(&mut ratings, Some(2), false, 6500, 5000);
        let recap = session_recap(&history(&reports, 2), Some(&ratings));
        assert_eq!(recap.games, 9);
        assert_eq!(recap.tiles.len(), TILES);
        assert_eq!(recap.tiles[0].hero.as_ref().unwrap().id, "H_4");
        assert_eq!(recap.tiles[7].hero.as_ref().unwrap().id, "H_11");
        assert_eq!(
            recap.starting.solo, None,
            "game 2's reading follows no reading of game 1"
        );
    }

    #[test]
    fn a_game_the_app_has_no_reading_for_says_no_change() {
        let reports = [report(SOLO, "H_A", 2, "ok"), report(SOLO, "H_B", 6, "ok")];
        let mut ratings = SavedRatings::open(&dir("gap")).unwrap();
        take(&mut ratings, Some(2), false, 6415, 5800);
        let recap = session_recap(&history(&reports, 0), Some(&ratings));
        assert_eq!(recap.tiles[1].change, None, "nothing was read after game 1");
        assert_eq!(recap.starting, Starting::default());
    }

    #[test]
    fn a_dev_tool_game_is_not_a_tile_but_the_game_after_it_is_measured_from_it() {
        let reports = [
            report(SOLO, "H_A", 2, "ok"),
            report(SOLO, "H_DEV", 1, "ok"),
            report(SOLO, "H_C", 3, "ok"),
        ];
        let mut ratings = SavedRatings::open(&dir("dev")).unwrap();
        take(&mut ratings, None, false, 6400, 5800);
        take(&mut ratings, Some(1), false, 6440, 5800);
        take(&mut ratings, Some(2), false, 6500, 5800);
        take(&mut ratings, Some(3), false, 6480, 5800);
        let mut all = history(&reports, 0);
        all[1].counted = false;
        let recap = session_recap(&all, Some(&ratings));
        let seen: Vec<_> = recap
            .tiles
            .iter()
            .map(|t| (t.hero.as_ref().unwrap().id.as_str(), t.change))
            .collect();
        assert_eq!(seen, [("H_A", Some(40)), ("H_C", Some(-20))]);
        assert_eq!(recap.games, 2);
    }

    #[test]
    fn a_day_with_no_games_has_nothing() {
        let reports = [report(SOLO, "H_A", 2, "ok")];
        let recap = session_recap(&history(&reports, 5), None);
        assert_eq!(recap, SessionRecap::default());
    }

    #[test]
    fn games_of_other_types_are_not_tiles() {
        let reports = [
            report("GT_OTHER", "H_A", 2, "ok"),
            report(SOLO, "H_B", 6, "ok"),
        ];
        let recap = session_recap(&history(&reports, 0), None);
        assert_eq!(recap.tiles.len(), 1);
        assert_eq!(recap.games, 1);
    }
}
