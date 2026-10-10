//! When each game of a session was played, from the log's own clock (T-D01).
//!
//! Every `Power.log` line starts with the local time of day ("D 21:15:03.12…")
//! and the session folder is named after the local date and time the game
//! client started (`Hearthstone_2026_10_10_21_03_45`). Together they give each
//! game's first and last line as a local date and time, turned into UTC with
//! the system's time zone rules. The span is saved next to the game's report,
//! never inside it, and is only used to find games played with the reconnect
//! dev tool (D-043). A session whose name or lines do not give a time has no
//! span: unknown, never a guess.

use std::collections::BTreeMap;

use bg_parser::CREATE_GAME;
use serde::{Deserialize, Serialize};

const SECS_PER_DAY: i64 = 86_400;
/// A time of day more than this earlier than the previous line means the
/// next day. Lines are written in order, so the only step back within a day
/// is the clock's one-hour daylight-saving fall-back (and tiny jitter); an
/// idle client left open overnight (22:00, then 10:30) must roll. Known
/// limit: a gap of almost a whole day or more cannot be told from the times.
const NEXT_DAY_STEP_BACK: i64 = 2 * 3600;

/// First and last line of a game, in seconds since 1970 (UTC).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Played {
    pub from: i64,
    pub to: i64,
}

/// Turns a local date and time (seconds since 1970 as if the local clock
/// were UTC) into real UTC seconds; `None` when it cannot.
pub type LocalToUtc = fn(i64) -> Option<i64>;

/// Follows one session's lines and remembers when each game started and
/// last wrote a line. Games are numbered like [`bg_parser::LogReader`]: the
/// n-th CREATE_GAME is game n.
#[derive(Debug)]
pub struct GameClock {
    /// Local midnight of the session's first day; `None` when the folder
    /// name gives no date.
    midnight: Option<i64>,
    day: i64,
    last_time_of_day: i64,
    count: usize,
    /// Game number -> (first, last) local seconds.
    games: BTreeMap<usize, (i64, i64)>,
}

impl GameClock {
    pub fn new(session: &str) -> Self {
        let start = session_start(session);
        GameClock {
            midnight: start.map(|s| s - s.rem_euclid(SECS_PER_DAY)),
            day: 0,
            last_time_of_day: start.map_or(0, |s| s.rem_euclid(SECS_PER_DAY)),
            count: 0,
            games: BTreeMap::new(),
        }
    }

    pub fn feed(&mut self, line: &str) {
        let is_new_game = line.contains(CREATE_GAME);
        if is_new_game {
            self.count += 1;
        }
        let Some(now) = self.local_time(line) else {
            return;
        };
        if is_new_game {
            self.games.insert(self.count, (now, now));
        } else if let Some(span) = self.games.get_mut(&self.count) {
            span.1 = now;
        }
    }

    fn local_time(&mut self, line: &str) -> Option<i64> {
        let midnight = self.midnight?;
        let time_of_day = line_time_of_day(line)?;
        if time_of_day + NEXT_DAY_STEP_BACK < self.last_time_of_day {
            self.day += 1;
        }
        self.last_time_of_day = time_of_day;
        Some(midnight + self.day * SECS_PER_DAY + time_of_day)
    }

    /// When game `index` was played, in UTC; `None` when unknown.
    pub fn played(&self, index: usize, to_utc: LocalToUtc) -> Option<Played> {
        let &(from, to) = self.games.get(&index)?;
        Some(Played {
            from: to_utc(from)?,
            to: to_utc(to)?,
        })
    }
}

/// "Hearthstone_2026_10_10_21_03_45" -> local seconds since 1970.
pub fn session_start(name: &str) -> Option<i64> {
    let rest = name.strip_prefix("Hearthstone_")?;
    let parts: Vec<i64> = rest
        .split('_')
        .map(|p| {
            (!p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
                .then(|| p.parse().ok())
                .flatten()
        })
        .collect::<Option<_>>()?;
    let [year, month, day, hour, minute, second] = parts[..] else {
        return None;
    };
    let valid = (1970..=9999).contains(&year)
        && (1..=12).contains(&month)
        && (1..=days_in_month(year, month)).contains(&day)
        && hour < 24
        && minute < 60
        && second < 60;
    valid.then(|| {
        days_from_civil(year, month, day) * SECS_PER_DAY + hour * 3600 + minute * 60 + second
    })
}

/// "D 21:15:03.1234567 …" -> seconds since midnight (fraction dropped).
fn line_time_of_day(line: &str) -> Option<i64> {
    let mut parts = line.splitn(3, ' ');
    if !matches!(parts.next()?, "D" | "W" | "E") {
        return None;
    }
    let time = parts.next()?;
    let whole = time.split('.').next()?;
    let mut fields = whole.split(':').map(|f| {
        (f.len() == 2 && f.bytes().all(|b| b.is_ascii_digit()))
            .then(|| f.parse::<i64>().ok())
            .flatten()
    });
    let (h, m, s) = (fields.next()??, fields.next()??, fields.next()??);
    (fields.next().is_none() && h < 24 && m < 60 && s < 60).then_some(h * 3600 + m * 60 + s)
}

fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        2 if is_leap(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Days since 1970-01-01 of a date in the proleptic Gregorian calendar
/// (Howard Hinnant's public-domain `days_from_civil` algorithm).
pub fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Inverse of [`days_from_civil`]: (year, month, day).
pub fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// The system's time zone rules, daylight saving included
/// (`TzSpecificLocalTimeToSystemTime`). Not available off Windows: unknown.
pub fn system_local_to_utc(local: i64) -> Option<i64> {
    windows_tz::local_to_utc(local)
}

#[cfg(windows)]
mod windows_tz {
    use super::{civil_from_days, days_from_civil, SECS_PER_DAY};

    /// SYSTEMTIME (minwinbase.h).
    #[repr(C)]
    #[derive(Default)]
    struct SystemTime {
        year: u16,
        month: u16,
        day_of_week: u16,
        day: u16,
        hour: u16,
        minute: u16,
        second: u16,
        milliseconds: u16,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn TzSpecificLocalTimeToSystemTime(
            time_zone: *const std::ffi::c_void,
            local: *const SystemTime,
            universal: *mut SystemTime,
        ) -> i32;
    }

    pub fn local_to_utc(local: i64) -> Option<i64> {
        let days = local.div_euclid(SECS_PER_DAY);
        let secs = local.rem_euclid(SECS_PER_DAY);
        let (year, month, day) = civil_from_days(days);
        let input = SystemTime {
            year: u16::try_from(year).ok()?,
            month: month as u16,
            day: day as u16,
            hour: (secs / 3600) as u16,
            minute: (secs / 60 % 60) as u16,
            second: (secs % 60) as u16,
            ..SystemTime::default()
        };
        let mut output = SystemTime::default();
        // SAFETY: both pointers are to live, properly laid out SYSTEMTIME
        // values for the duration of the call; a null time zone means "the
        // active one" (Microsoft's documentation of the function).
        let ok = unsafe { TzSpecificLocalTimeToSystemTime(std::ptr::null(), &input, &mut output) };
        if ok == 0 {
            return None;
        }
        let days = days_from_civil(
            i64::from(output.year),
            i64::from(output.month),
            i64::from(output.day),
        );
        Some(
            days * SECS_PER_DAY
                + i64::from(output.hour) * 3600
                + i64::from(output.minute) * 60
                + i64::from(output.second),
        )
    }
}

#[cfg(not(windows))]
mod windows_tz {
    pub fn local_to_utc(_local: i64) -> Option<i64> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SESSION: &str = "Hearthstone_2026_10_10_21_03_45";

    fn utc_plus_2(local: i64) -> Option<i64> {
        Some(local - 2 * 3600)
    }

    fn line(time: &str, rest: &str) -> String {
        format!("D {time}.1234567 {rest}")
    }

    fn create(time: &str) -> String {
        line(time, CREATE_GAME)
    }

    fn at(y: i64, mo: i64, d: i64, h: i64, mi: i64, s: i64) -> i64 {
        days_from_civil(y, mo, d) * SECS_PER_DAY + h * 3600 + mi * 60 + s
    }

    #[test]
    fn civil_dates_round_trip() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(2000, 3, 1), 11_017);
        for days in [-1, 0, 59, 60, 11_016, 20_736, 40_000] {
            let (y, m, d) = civil_from_days(days);
            assert_eq!(days_from_civil(y, m, d), days);
        }
        assert_eq!(civil_from_days(days_from_civil(2028, 2, 29)), (2028, 2, 29));
    }

    #[test]
    fn the_session_name_gives_the_local_start() {
        assert_eq!(session_start(SESSION), Some(at(2026, 10, 10, 21, 3, 45)));
        assert_eq!(session_start("Hearthstone_2026_02_30_00_00_00"), None);
        assert_eq!(session_start("Hearthstone_2026_10_10_24_00_00"), None);
        assert_eq!(session_start("Hearthstone_2026_10_10_21_03"), None);
        assert_eq!(session_start("Hearthstone_2026_10_10_21_03_+5"), None);
        assert_eq!(session_start("replay"), None);
        assert_eq!(
            session_start("Hearthstone_99999999999999_10_10_21_03_45"),
            None
        );
        assert_eq!(session_start("Hearthstone_1969_12_31_23_00_00"), None);
    }

    #[test]
    fn each_game_spans_from_its_create_game_to_its_last_line() {
        let mut clock = GameClock::new(SESSION);
        clock.feed(&line(
            "21:04:00",
            "GameState.DebugPrintGame() - BuildNumber=1",
        ));
        clock.feed(&create("21:10:00"));
        clock.feed(&line("21:20:30", "TAG_CHANGE"));
        clock.feed(&create("21:30:00"));
        clock.feed(&line("21:45:10", "TAG_CHANGE"));
        let first = clock.played(1, utc_plus_2).unwrap();
        assert_eq!(first.from, at(2026, 10, 10, 19, 10, 0));
        assert_eq!(first.to, at(2026, 10, 10, 19, 20, 30));
        let second = clock.played(2, utc_plus_2).unwrap();
        assert_eq!(second.from, at(2026, 10, 10, 19, 30, 0));
        assert_eq!(second.to, at(2026, 10, 10, 19, 45, 10));
        assert_eq!(clock.played(3, utc_plus_2), None);
    }

    #[test]
    fn a_game_across_midnight_ends_the_next_day() {
        let mut clock = GameClock::new("Hearthstone_2026_10_10_23_50_00");
        clock.feed(&create("23:55:00"));
        clock.feed(&line("23:59:59", "TAG_CHANGE"));
        clock.feed(&line("23:59:58", "a line a moment back is not a new day"));
        clock.feed(&line("00:10:00", "TAG_CHANGE"));
        let played = clock.played(1, Some).unwrap();
        assert_eq!(played.from, at(2026, 10, 10, 23, 55, 0));
        assert_eq!(played.to, at(2026, 10, 11, 0, 10, 0));
    }

    #[test]
    fn a_client_left_open_overnight_dates_the_next_game_the_next_day() {
        let mut clock = GameClock::new("Hearthstone_2026_10_10_18_00_00");
        clock.feed(&create("21:00:00"));
        clock.feed(&line("22:00:00", "TAG_CHANGE"));
        clock.feed(&create("10:30:00"));
        assert_eq!(
            clock.played(2, Some).unwrap().from,
            at(2026, 10, 11, 10, 30, 0)
        );
    }

    #[test]
    fn the_daylight_saving_fall_back_hour_is_not_a_new_day() {
        let mut clock = GameClock::new("Hearthstone_2026_10_25_01_00_00");
        clock.feed(&create("02:50:00"));
        clock.feed(&line("02:05:00", "the clock went back one hour"));
        assert_eq!(clock.played(1, Some).unwrap().to, at(2026, 10, 25, 2, 5, 0));
    }

    #[test]
    fn a_session_started_just_before_midnight_counts_its_first_lines_the_next_day() {
        let mut clock = GameClock::new("Hearthstone_2026_10_10_23_59_50");
        clock.feed(&create("00:00:05"));
        assert_eq!(
            clock.played(1, Some).unwrap().from,
            at(2026, 10, 11, 0, 0, 5)
        );
    }

    #[test]
    fn without_a_session_date_or_a_time_zone_the_span_is_unknown() {
        let mut clock = GameClock::new("my-replay");
        clock.feed(&create("21:10:00"));
        assert_eq!(clock.played(1, Some), None);

        let mut clock = GameClock::new(SESSION);
        clock.feed(&create("21:10:00"));
        assert_eq!(clock.played(1, |_| None), None);
    }

    #[test]
    fn lines_without_a_time_keep_the_game_count_and_change_no_span() {
        let mut clock = GameClock::new(SESSION);
        clock.feed(&format!("garbage {CREATE_GAME}"));
        clock.feed(&create("21:10:00"));
        clock.feed("garbage line without a timestamp");
        clock.feed(&line("25:00:00", "impossible hour"));
        assert_eq!(clock.played(1, Some), None);
        let second = clock.played(2, Some).unwrap();
        assert_eq!(
            (second.from, second.to),
            (at(2026, 10, 10, 21, 10, 0), at(2026, 10, 10, 21, 10, 0))
        );
    }

    #[cfg(windows)]
    #[test]
    fn the_system_time_zone_converts_a_local_time() {
        let local = at(2026, 7, 1, 12, 0, 0);
        let utc = system_local_to_utc(local).expect("Windows converts local times");
        // Every real time zone is within 14 hours of UTC.
        assert!((utc - local).abs() <= 14 * 3600);
    }
}
