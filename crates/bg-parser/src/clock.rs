//! A game's own clock: each line's time of day as milliseconds from the
//! game's first line (T-205). Never a clock time: only durations leave here.
//!
//! Real logs print times to 100 ns ("12:34:56.7890123"). Differences are
//! taken at that precision and only then cut to whole ms, so a fixture whose
//! times were moved to start at 00:00:00 reads exactly like the raw log.

const TICKS_PER_MS: i64 = 10_000;
const DAY_TICKS: i64 = 24 * 60 * 60 * 1000 * TICKS_PER_MS;

/// A time earlier than the last one by more than half a day is the next day.
#[derive(Debug, Default)]
pub struct Clock {
    first: Option<i64>,
    last: i64,
    days: i64,
}

impl Clock {
    /// Ms since the game's first timed line; None for a line without a time.
    pub fn at(&mut self, line: &str) -> Option<i64> {
        let ticks = time_of_day_ticks(line)?;
        let first = *self.first.get_or_insert(ticks);
        if ticks < self.last - DAY_TICKS / 2 {
            self.days += 1;
        }
        self.last = ticks;
        Some((ticks + self.days * DAY_TICKS - first).max(0) / TICKS_PER_MS)
    }
}

/// "D 12:34:56.7890123 ..." -> 100 ns ticks since midnight.
fn time_of_day_ticks(line: &str) -> Option<i64> {
    let time = line.split(' ').nth(1)?;
    let (hms, fraction) = time.split_once('.').unwrap_or((time, ""));
    let mut parts = hms.split(':').map(|p| p.parse::<i64>().ok());
    let (h, m, s) = (parts.next()??, parts.next()??, parts.next()??);
    let digits: String = fraction.chars().take(7).collect();
    if parts.next().is_some()
        || !(0..24).contains(&h)
        || !(0..60).contains(&m)
        || !(0..60).contains(&s)
        || !digits.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let fraction: i64 = format!("{digits:0<7}").parse().ok()?;
    Some(((h * 60 + m) * 60 + s) * 1000 * TICKS_PER_MS + fraction)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(time: &str) -> String {
        format!("D {time} GameState.DebugPrintPower() - BLOCK_END")
    }

    #[test]
    fn counts_from_the_first_line_at_full_precision() {
        let mut clock = Clock::default();
        assert_eq!(clock.at(&line("12:00:00.9597443")), Some(0));
        // 46.7429999 ms later is 46 ms, not 47 - 1.
        assert_eq!(clock.at(&line("12:00:47.7027442")), Some(46_742));
        assert_eq!(clock.at(&line("12:01:00")), Some(59_040));
    }

    #[test]
    fn past_midnight_keeps_counting() {
        let mut clock = Clock::default();
        assert_eq!(clock.at(&line("23:59:59.0000000")), Some(0));
        assert_eq!(clock.at(&line("00:00:01.0000000")), Some(2_000));
        // A small step back is not a new day.
        assert_eq!(clock.at(&line("00:00:00.5000000")), Some(1_500));
    }

    #[test]
    fn a_line_without_a_time_gives_none() {
        let mut clock = Clock::default();
        for bad in ["garbage", "D 25:00:00.0 x", "D 12:00:00.12a x", "D 12:00 x"] {
            assert_eq!(clock.at(bad), None, "{bad}");
        }
    }
}
