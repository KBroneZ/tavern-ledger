//! What the main window's "Today / this week" summary shows (T-361): the
//! summary of the page's local day and of its local week (Monday to Monday),
//! per mode. The page renders it and decides nothing.
//!
//! The page sends the UTC seconds of its own local midnights, so the day and
//! the week follow the viewer's time zone and its clock changes; this side
//! only checks they are a real day and a real week.

use serde::Serialize;
use tracker::period_summary::ModePeriod;

/// A week is 7 days, or an hour more or less around a clock change; anything
/// longer is not a week the page asked for.
const MAX_WEEK: i64 = 7 * 86_400 + 3600;
const MIN_WEEK: i64 = 7 * 86_400 - 3600;

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct PeriodView {
    /// The local day, Solo and Duos apart; only modes played.
    pub today: Vec<ModePeriod>,
    /// The local week from Monday 00:00; only modes played.
    pub week: Vec<ModePeriod>,
}

/// Whether the page's week (`start`..`end`, UTC seconds of the viewer's local
/// midnights of two Mondays) is one: 7 days long, give or take a clock change.
pub fn sane_week(start: i64, end: i64) -> bool {
    end.checked_sub(start)
        .is_some_and(|span| (MIN_WEEK..=MAX_WEEK).contains(&span))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lobby_panel::played_in;
    use tracker::game_clock::days_from_civil;

    /// UTC+3 all year, so the fixture does not depend on this PC's zone.
    fn utc_plus_3(local: i64) -> Option<i64> {
        Some(local - 3 * 3600)
    }

    /// A named local time of the UTC+3 fixture, as UTC seconds.
    fn at(y: i64, m: i64, d: i64, h: i64, min: i64, s: i64) -> i64 {
        days_from_civil(y, m, d) * 86_400 + h * 3600 + min * 60 + s - 3 * 3600
    }

    // Monday 2026-10-19 to Monday 2026-10-26, local midnights.
    fn week() -> (i64, i64) {
        (at(2026, 10, 19, 0, 0, 0), at(2026, 10, 26, 0, 0, 0))
    }

    #[test]
    fn a_week_is_seven_days_give_or_take_a_clock_change() {
        let (start, end) = week();
        assert!(sane_week(start, end));
        assert!(sane_week(start, end + 3600), "the clock went back");
        assert!(sane_week(start, end - 3600), "the clock went forward");
        assert!(!sane_week(start, start + 86_400), "a day");
        assert!(!sane_week(start, start + 30 * 86_400), "a month");
        assert!(!sane_week(end, start), "backwards");
        assert!(!sane_week(-(1 << 62), 1 << 62), "no overflow");
    }

    #[test]
    fn the_week_starts_on_monday_at_local_midnight_and_ends_the_next_monday() {
        let w = week();
        let s = "Hearthstone_2026_10_18_20_00_00";
        // Sunday 23:59:59 local belongs to the week before; Monday 00:00:00 to this one.
        assert!(!played_in(
            Some(at(2026, 10, 18, 23, 59, 59)),
            s,
            w,
            utc_plus_3
        ));
        assert!(played_in(Some(at(2026, 10, 19, 0, 0, 0)), s, w, utc_plus_3));
        assert!(played_in(
            Some(at(2026, 10, 25, 23, 59, 59)),
            s,
            w,
            utc_plus_3
        ));
        assert!(!played_in(
            Some(at(2026, 10, 26, 0, 0, 0)),
            s,
            w,
            utc_plus_3
        ));
    }

    #[test]
    fn a_game_with_no_end_time_counts_by_its_session_start_in_local_time() {
        let w = week();
        let in_week = |session| played_in(None, session, w, utc_plus_3);
        assert!(!in_week("Hearthstone_2026_10_18_23_59_59"));
        assert!(in_week("Hearthstone_2026_10_19_00_00_00"));
        assert!(in_week("Hearthstone_2026_10_25_23_59_59"));
        assert!(!in_week("Hearthstone_2026_10_26_00_00_00"));
    }

    #[test]
    fn a_game_just_before_local_midnight_is_still_the_same_utc_day_elsewhere() {
        // 2026-10-19 01:30 local is 2026-10-18 22:30 UTC: the local week owns
        // it even though the UTC date is still the Sunday.
        let w = week();
        assert!(played_in(
            Some(at(2026, 10, 19, 1, 30, 0)),
            "Hearthstone_2026_10_19_01_00_00",
            w,
            utc_plus_3
        ));
        let utc_sunday_22_30 = days_from_civil(2026, 10, 18) * 86_400 + 22 * 3600 + 30 * 60;
        assert_eq!(at(2026, 10, 19, 1, 30, 0), utc_sunday_22_30);
    }
}
