//! The "Today / this week" summary of the main window (T-361): per mode, the
//! games of a stretch of time with their average place and record, the net
//! rating change, and up to three games worth revisiting. The window renders
//! it and decides nothing.
//!
//! The numbers are the ones the lobby panel already shows ([`stats::summary`]
//! for games, average place and record; [`SavedRatings`] for the rating), asked
//! for a day and for a week. `in_day` of each [`HistoryGame`] says whether the
//! game was played in the stretch the summary is about.
//!
//! The net rating change is its own state, never a number made up (D-011,
//! D-063, D-149): `off` when the read from the game is off, `unknown` when it
//! is on but the rating is missing or not known to be current at one end of the
//! stretch, and a number only when both ends are known. A net change of zero is
//! a known value and is not the same as unknown.
//!
//! No streaks (D-172).

use serde::Serialize;

use crate::game_ratings::SavedRatings;
use crate::lobby_recap::HistoryGame;
use crate::recap::{text, HeroRef, View};
use crate::stats::{counted_place, mode_of, summary, ModeSummary, DUOS, OTHER};
use crate::store::GameKey;

/// How many games the summary suggests revisiting.
pub const REVISIT: usize = 3;

/// A "big swing" is a place at least this many places away from the average
/// place of the stretch.
const SWING_PLACES: i64 = 2;

/// The net rating change of one mode over the stretch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "state", content = "change", rename_all = "snake_case")]
pub enum NetRating {
    /// The read from the game is off: there is no rating to compare.
    Off,
    /// The read is on, but one end is missing or may be out of date.
    Unknown,
    /// The rating at the end minus the rating at the start (may be zero).
    Known(i32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    /// The best place of the stretch.
    BestPlace,
    /// Finished just outside the top half.
    CloseLoss,
    /// A place far from the stretch's average.
    BigSwing,
}

/// A game worth another look; the window opens its replay from the key.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Revisit {
    pub reason: Reason,
    pub session: String,
    pub index: u64,
    pub hero: Option<HeroRef>,
    pub place: i64,
    /// The rating this game won or lost; None when the app does not know it.
    pub change: Option<i32>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ModePeriod {
    /// Games, average place and record (top half out of placed games).
    pub summary: ModeSummary,
    pub net_rating: NetRating,
    pub revisit: Vec<Revisit>,
}

/// A counted game of one mode with a counted place.
struct Placed<'a> {
    at: usize,
    game: &'a HistoryGame<'a>,
    place: i64,
}

/// The summary of the games with `in_day` set, Solo then Duos, only the modes
/// with a game. `history` is every game the app has seen, oldest first (the
/// reconnect dev tool's games are in it, `counted` false, because they moved
/// the rating). `ratings` is None when the read from the game is off.
pub fn period_summary(history: &[HistoryGame], ratings: Option<&SavedRatings>) -> Vec<ModePeriod> {
    let counted = || {
        history
            .iter()
            .enumerate()
            .filter(|(_, g)| g.in_day && g.counted && mode_of(g.report) != OTHER)
    };
    let ends = rating_ends(history);
    summary(counted().map(|(_, g)| g.report))
        .into_iter()
        .map(|summary| {
            let mode = summary.mode.as_str();
            let placed: Vec<Placed> = counted()
                .filter(|(_, g)| mode_of(g.report) == mode)
                .filter_map(|(at, game)| {
                    let status = text(game.report, "status");
                    let place = counted_place(mode, status.as_deref(), game.report)?;
                    Some(Placed { at, game, place })
                })
                .collect();
            ModePeriod {
                net_rating: net_rating(mode == DUOS, ends, history, ratings),
                revisit: revisit(&placed, summary.top_half, history, ratings),
                summary,
            }
        })
        .collect()
}

/// The first and last game played in the stretch, the dev tool's included.
fn rating_ends(history: &[HistoryGame]) -> Option<(usize, usize)> {
    let first = history.iter().position(|g| g.in_day)?;
    let last = history.iter().rposition(|g| g.in_day)?;
    Some((first, last))
}

fn previous<'a>(history: &'a [HistoryGame], at: usize) -> Option<&'a GameKey> {
    at.checked_sub(1).map(|i| &history[i].key)
}

fn net_rating(
    duos: bool,
    ends: Option<(usize, usize)>,
    history: &[HistoryGame],
    ratings: Option<&SavedRatings>,
) -> NetRating {
    let Some(ratings) = ratings else {
        return NetRating::Off;
    };
    let Some((first, last)) = ends else {
        return NetRating::Unknown;
    };
    let (solo, duos_start) = ratings.starting(previous(history, first));
    let start = if duos { duos_start } else { solo };
    let end = ratings
        .after(Some(&history[last].key))
        .filter(|r| r.fresh(duos))
        .and_then(|r| r.of(duos));
    match (start, end) {
        (Some(start), Some(end)) => NetRating::Known(end as i32 - start as i32),
        _ => NetRating::Unknown,
    }
}

/// Best place, a close loss, then a big swing: each a different game, newest
/// first when two tie.
fn revisit(
    placed: &[Placed],
    top_half: i64,
    history: &[HistoryGame],
    ratings: Option<&SavedRatings>,
) -> Vec<Revisit> {
    let mut chosen: Vec<(Reason, &Placed)> = Vec::new();
    let free = |chosen: &[(Reason, &Placed)], p: &Placed| chosen.iter().all(|(_, c)| c.at != p.at);

    let best = placed
        .iter()
        .min_by(|a, b| a.place.cmp(&b.place).then(b.at.cmp(&a.at)));
    chosen.extend(best.map(|p| (Reason::BestPlace, p)));

    let close = placed
        .iter()
        .rev()
        .find(|p| p.place == top_half + 1 && free(&chosen, p));
    chosen.extend(close.map(|p| (Reason::CloseLoss, p)));

    // Distance from the average, in units of 1/n place, to stay in integers.
    let n = placed.len() as i64;
    let sum: i64 = placed.iter().map(|p| p.place).sum();
    let distance = |p: &Placed| (p.place * n - sum).abs();
    let swing = placed
        .iter()
        .filter(|p| free(&chosen, p) && distance(p) >= SWING_PLACES * n)
        .max_by(|a, b| distance(a).cmp(&distance(b)).then(a.at.cmp(&b.at)));
    chosen.extend(swing.map(|p| (Reason::BigSwing, p)));

    chosen
        .into_iter()
        .take(REVISIT)
        .map(|(reason, p)| {
            let duos = mode_of(p.game.report) == DUOS;
            Revisit {
                reason,
                session: p.game.key.session.clone(),
                index: p.game.key.index,
                hero: text(p.game.report, "hero").map(|id| View::new(p.game.report).hero_ref(&id)),
                place: p.place,
                change: ratings.and_then(|r| r.change(&p.game.key, previous(history, p.at), duos)),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_ratings::Taken;
    use crate::stats::SOLO;
    use serde_json::{json, Value};
    use std::path::PathBuf;

    fn dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tl-period-{tag}-{}-{:?}",
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
    /// being Duos or not.
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

    fn history(reports: &[Value], first_in_period: usize) -> Vec<HistoryGame<'_>> {
        reports
            .iter()
            .enumerate()
            .map(|(i, report)| HistoryGame {
                key: key(i as u64 + 1),
                report,
                in_day: i >= first_in_period,
                counted: true,
            })
            .collect()
    }

    fn solo_games(places: &[i64]) -> Vec<Value> {
        places
            .iter()
            .enumerate()
            .map(|(i, p)| report(SOLO, &format!("H_{}", i + 1), *p, "ok"))
            .collect()
    }

    #[test]
    fn games_average_place_and_record_come_per_mode() {
        let reports = [
            report(SOLO, "H_A", 1, "ok"),
            report(SOLO, "H_B", 5, "ok"),
            report(SOLO, "H_C", 3, "ok"),
            report(DUOS, "H_D", 2, "ok"),
        ];
        let modes = period_summary(&history(&reports, 0), None);
        assert_eq!(modes.len(), 2);
        let solo = &modes[0].summary;
        assert_eq!(solo.mode, SOLO);
        assert_eq!(solo.tally.games, 3);
        assert_eq!(solo.tally.average_place, Some(3.0));
        assert_eq!((solo.top_half_games, solo.tally.placed), (2, 3));
        assert_eq!(solo.top_half, 4);
        let duos = &modes[1].summary;
        assert_eq!(duos.mode, DUOS);
        assert_eq!(duos.tally.average_place, Some(2.0));
        assert_eq!((duos.top_half_games, duos.tally.placed), (1, 1));
        assert_eq!(duos.top_half, 2);
    }

    #[test]
    fn a_stretch_with_no_games_has_no_modes() {
        let reports = solo_games(&[1, 2]);
        assert!(period_summary(&history(&reports, 5), None).is_empty());
        assert!(period_summary(&[], None).is_empty());
    }

    #[test]
    fn only_games_in_the_stretch_count() {
        let reports = solo_games(&[8, 8, 2]);
        let modes = period_summary(&history(&reports, 2), None);
        assert_eq!(modes[0].summary.tally.games, 1);
        assert_eq!(modes[0].summary.tally.average_place, Some(2.0));
    }

    #[test]
    fn only_the_mode_played_shows() {
        let reports = [report(DUOS, "H_A", 3, "ok")];
        let modes = period_summary(&history(&reports, 0), None);
        assert_eq!(modes.len(), 1);
        assert_eq!(modes[0].summary.mode, DUOS);
    }

    #[test]
    fn unfinished_games_and_other_types_have_no_place_and_no_revisit() {
        let reports = [
            report(SOLO, "H_A", 1, "incomplete"),
            report("GT_OTHER", "H_B", 1, "ok"),
        ];
        let modes = period_summary(&history(&reports, 0), None);
        assert_eq!(modes.len(), 1);
        let solo = &modes[0];
        assert_eq!(solo.summary.tally.games, 1);
        assert_eq!(solo.summary.tally.average_place, None);
        assert!(solo.revisit.is_empty());
    }

    #[test]
    fn with_the_rating_option_off_the_net_change_is_off_never_zero() {
        let reports = solo_games(&[2, 6]);
        let modes = period_summary(&history(&reports, 0), None);
        assert_eq!(modes[0].net_rating, NetRating::Off);
        assert!(modes[0].revisit.iter().all(|r| r.change.is_none()));
    }

    #[test]
    fn the_net_change_is_the_rating_at_the_end_minus_the_one_at_the_start() {
        let reports = [
            report(SOLO, "H_A", 2, "ok"),
            report(DUOS, "H_B", 1, "ok"),
            report(SOLO, "H_C", 6, "ok"),
        ];
        let mut ratings = SavedRatings::open(&dir("net")).unwrap();
        take(&mut ratings, None, false, 6400, 5800);
        take(&mut ratings, Some(1), false, 6440, 5800);
        take(&mut ratings, Some(2), true, 6440, 5850);
        take(&mut ratings, Some(3), false, 6415, 5850);
        let modes = period_summary(&history(&reports, 0), Some(&ratings));
        assert_eq!(modes[0].net_rating, NetRating::Known(15));
        assert_eq!(modes[1].net_rating, NetRating::Known(50));
    }

    #[test]
    fn a_net_change_that_nets_out_to_zero_is_known_not_unknown() {
        let reports = solo_games(&[2, 6]);
        let mut ratings = SavedRatings::open(&dir("zero")).unwrap();
        take(&mut ratings, None, false, 6400, 5800);
        take(&mut ratings, Some(1), false, 6440, 5800);
        take(&mut ratings, Some(2), false, 6400, 5800);
        let modes = period_summary(&history(&reports, 0), Some(&ratings));
        assert_eq!(modes[0].net_rating, NetRating::Known(0));
    }

    #[test]
    fn a_missing_rating_at_the_start_makes_the_net_change_unknown() {
        let reports = solo_games(&[2, 6]);
        let mut ratings = SavedRatings::open(&dir("no-start")).unwrap();
        // Nothing was read before game 1.
        take(&mut ratings, Some(1), false, 6440, 5800);
        take(&mut ratings, Some(2), false, 6415, 5800);
        let modes = period_summary(&history(&reports, 0), Some(&ratings));
        assert_eq!(modes[0].net_rating, NetRating::Unknown);
    }

    #[test]
    fn a_missing_rating_at_the_end_makes_the_net_change_unknown() {
        let reports = solo_games(&[2, 6]);
        let mut ratings = SavedRatings::open(&dir("no-end")).unwrap();
        take(&mut ratings, None, false, 6400, 5800);
        take(&mut ratings, Some(1), false, 6440, 5800);
        let modes = period_summary(&history(&reports, 0), Some(&ratings));
        assert_eq!(modes[0].net_rating, NetRating::Unknown);
    }

    #[test]
    fn a_rating_that_may_be_out_of_date_at_the_end_is_unknown() {
        let reports = solo_games(&[2]);
        let mut ratings = SavedRatings::open(&dir("stale")).unwrap();
        take(&mut ratings, None, false, 6400, 5800);
        // The game was Solo and the Solo rating did not move: the game may
        // still have been showing the old one.
        take(&mut ratings, Some(1), false, 6400, 5800);
        let modes = period_summary(&history(&reports, 0), Some(&ratings));
        assert_eq!(modes[0].net_rating, NetRating::Unknown);
    }

    #[test]
    fn a_dev_tool_game_moves_the_rating_but_is_not_counted() {
        let reports = [report(SOLO, "H_DEV", 1, "ok"), report(SOLO, "H_A", 3, "ok")];
        let mut ratings = SavedRatings::open(&dir("dev")).unwrap();
        take(&mut ratings, None, false, 6400, 5800);
        take(&mut ratings, Some(1), false, 6460, 5800);
        take(&mut ratings, Some(2), false, 6480, 5800);
        let mut all = history(&reports, 0);
        all[0].counted = false;
        let modes = period_summary(&all, Some(&ratings));
        assert_eq!(modes[0].summary.tally.games, 1);
        assert_eq!(modes[0].net_rating, NetRating::Known(80));
    }

    #[test]
    fn the_net_change_serialises_with_its_state_and_a_number_only_when_known() {
        let v = |n| serde_json::to_value(n).unwrap();
        assert_eq!(v(NetRating::Off), json!({"state": "off"}));
        assert_eq!(v(NetRating::Unknown), json!({"state": "unknown"}));
        assert_eq!(
            v(NetRating::Known(-12)),
            json!({"state": "known", "change": -12})
        );
        assert_eq!(
            v(NetRating::Known(0)),
            json!({"state": "known", "change": 0})
        );
    }

    #[test]
    fn revisit_picks_the_best_place_a_close_loss_and_a_big_swing_in_that_order() {
        // Average 4.5: the 8 is the only place two or more away from it that
        // is not the best or the close loss.
        let reports = solo_games(&[4, 8, 5, 1]);
        let modes = period_summary(&history(&reports, 0), None);
        let seen: Vec<_> = modes[0]
            .revisit
            .iter()
            .map(|r| (r.reason, r.index, r.place))
            .collect();
        assert_eq!(
            seen,
            [
                (Reason::BestPlace, 4, 1),
                (Reason::CloseLoss, 3, 5),
                (Reason::BigSwing, 2, 8),
            ]
        );
        assert_eq!(modes[0].revisit[0].session, key(4).session);
        assert_eq!(
            modes[0].revisit[0].hero.as_ref().unwrap().name.as_deref(),
            Some("Name of H_4")
        );
    }

    #[test]
    fn a_game_that_fits_two_reasons_shows_once() {
        let reports = solo_games(&[5]);
        let modes = period_summary(&history(&reports, 0), None);
        let seen: Vec<_> = modes[0].revisit.iter().map(|r| r.reason).collect();
        assert_eq!(seen, [Reason::BestPlace]);
    }

    #[test]
    fn a_close_loss_in_duos_is_third_place() {
        let reports = [
            report(DUOS, "H_A", 1, "ok"),
            report(DUOS, "H_B", 3, "ok"),
            report(DUOS, "H_C", 4, "ok"),
        ];
        let modes = period_summary(&history(&reports, 0), None);
        let close: Vec<_> = modes[0]
            .revisit
            .iter()
            .filter(|r| r.reason == Reason::CloseLoss)
            .map(|r| r.index)
            .collect();
        assert_eq!(close, [2]);
    }

    #[test]
    fn ties_go_to_the_newest_game() {
        // Average 5: the 3s and the 7s are all two places from it.
        let reports = solo_games(&[3, 7, 3, 7, 5]);
        let modes = period_summary(&history(&reports, 0), None);
        let by = |reason| {
            modes[0]
                .revisit
                .iter()
                .find(|r| r.reason == reason)
                .map(|r| r.index)
        };
        assert_eq!(by(Reason::BestPlace), Some(3));
        assert_eq!(by(Reason::CloseLoss), Some(5));
        assert_eq!(by(Reason::BigSwing), Some(4));
    }

    #[test]
    fn no_swing_is_suggested_when_every_place_is_near_the_average() {
        let reports = solo_games(&[3, 4, 4, 3]);
        let modes = period_summary(&history(&reports, 0), None);
        let seen: Vec<_> = modes[0].revisit.iter().map(|r| r.reason).collect();
        assert_eq!(seen, [Reason::BestPlace]);
    }

    #[test]
    fn a_revisit_carries_its_rating_change_only_when_known() {
        let reports = solo_games(&[1, 6]);
        let mut ratings = SavedRatings::open(&dir("revisit")).unwrap();
        take(&mut ratings, None, false, 6400, 5800);
        take(&mut ratings, Some(1), false, 6440, 5800);
        // Nothing was read after game 2.
        let modes = period_summary(&history(&reports, 0), Some(&ratings));
        let best = &modes[0].revisit[0];
        assert_eq!((best.index, best.change), (1, Some(40)));
    }
}
