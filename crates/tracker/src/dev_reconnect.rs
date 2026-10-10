//! Games played with the reconnect dev tool (T-D01, D-022, D-043).
//!
//! `tools/dev/reconnect.py` appends one line per use to
//! `dev-reconnects.jsonl` in the data folder: `{"utc": "2026-10-10T19:15:03Z"}`,
//! nothing else. A game whose span ([`Played`]) holds one of those times is a
//! *dev reconnect* game: the history marks it and the stats and the upload
//! leave it out. The marks are worked out each time from that file, never
//! saved in the history. No file is the normal case (the tool was never
//! used): no marks. A file that cannot be read, or has lines that make no
//! sense, gives the marks it can and a warning; never a crash.

use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use serde_json::Value;

use crate::game_clock::{days_from_civil, Played};

pub const FILE_NAME: &str = "dev-reconnects.jsonl";
/// A use this close before a game's first line or after its last still
/// marks it: the log is written a moment after the game acts, and a wrong
/// mark only leaves a game out of the stats.
pub const MARGIN_SECS: i64 = 30;
/// The tool writes a few dozen bytes per use; anything bigger is not its file.
const MAX_FILE_BYTES: u64 = 4 << 20;

#[derive(Debug, Default, PartialEq)]
pub struct Reconnects {
    /// Seconds since 1970 (UTC), sorted.
    times: Vec<i64>,
    /// Why some or all of the file could not be used; `None` when it was
    /// read whole (or does not exist).
    pub warning: Option<String>,
}

impl Reconnects {
    /// Reads `dev-reconnects.jsonl` in `dir`.
    pub fn load(dir: &Path) -> Reconnects {
        match read_capped(&dir.join(FILE_NAME)) {
            Ok(Some(text)) => Reconnects::parse(&text),
            Ok(None) => Reconnects::default(),
            Err(e) => Reconnects {
                times: Vec::new(),
                warning: Some(format!(
                    "Could not read {FILE_NAME} ({:?}); no game is marked as played with the reconnect dev tool.",
                    e.kind()
                )),
            },
        }
    }

    pub fn parse(text: &str) -> Reconnects {
        let mut times = Vec::new();
        let mut bad = 0;
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            match parse_line(line) {
                Some(t) => times.push(t),
                None => bad += 1,
            }
        }
        times.sort_unstable();
        let warning = (bad > 0)
            .then(|| format!("{bad} line(s) of {FILE_NAME} could not be read and were skipped."));
        Reconnects { times, warning }
    }

    pub fn is_empty(&self) -> bool {
        self.times.is_empty()
    }

    /// True when the tool was used while the game was on. A game with no
    /// known span cannot be checked and is not marked.
    pub fn marks(&self, played: Option<&Played>) -> bool {
        let Some(p) = played else {
            return false;
        };
        let start = self.times.partition_point(|&t| t < p.from - MARGIN_SECS);
        self.times
            .get(start)
            .is_some_and(|&t| t <= p.to + MARGIN_SECS)
    }
}

fn read_capped(path: &Path) -> io::Result<Option<String>> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    if file.metadata()?.len() > MAX_FILE_BYTES {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "file too large"));
    }
    let mut bytes = Vec::new();
    file.take(MAX_FILE_BYTES).read_to_end(&mut bytes)?;
    Ok(Some(String::from_utf8_lossy(&bytes).into_owned()))
}

fn parse_line(line: &str) -> Option<i64> {
    let value: Value = serde_json::from_str(line).ok()?;
    parse_utc(value.get("utc")?.as_str()?)
}

/// "2026-10-10T19:15:03Z" (exactly this shape) -> seconds since 1970.
pub fn parse_utc(text: &str) -> Option<i64> {
    let b = text.as_bytes();
    let shape_ok = b.len() == 20
        && [4, 7].iter().all(|&i| b[i] == b'-')
        && b[10] == b'T'
        && [13, 16].iter().all(|&i| b[i] == b':')
        && b[19] == b'Z';
    if !shape_ok {
        return None;
    }
    let num = |r: std::ops::Range<usize>| -> Option<i64> {
        let part = &text[r];
        part.bytes()
            .all(|c| c.is_ascii_digit())
            .then(|| part.parse().ok())
            .flatten()
    };
    let (year, month, day) = (num(0..4)?, num(5..7)?, num(8..10)?);
    let (hour, minute, second) = (num(11..13)?, num(14..16)?, num(17..19)?);
    let days = days_from_civil(year, month, day);
    // Rejects 2026-02-30 and the like: the date must survive the round trip.
    let valid = crate::game_clock::civil_from_days(days) == (year, month, day)
        && hour < 24
        && minute < 60
        && second < 60;
    valid.then_some(days * 86_400 + hour * 3600 + minute * 60 + second)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span(from: &str, to: &str) -> Played {
        Played {
            from: parse_utc(from).unwrap(),
            to: parse_utc(to).unwrap(),
        }
    }

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("tl-devrc-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn utc_times_are_read_in_one_exact_shape() {
        assert_eq!(parse_utc("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse_utc("2026-10-10T19:15:03Z"), Some(1_791_659_703));
        for bad in [
            "2026-10-10 19:15:03Z",
            "2026-10-10T19:15:03",
            "2026-10-10T19:15:03+02:00",
            "2026-02-30T00:00:00Z",
            "2026-13-01T00:00:00Z",
            "2026-10-10T24:00:00Z",
            "2026-1a-10T19:15:03Z",
            "",
        ] {
            assert_eq!(parse_utc(bad), None, "{bad}");
        }
    }

    #[test]
    fn a_game_is_marked_when_a_use_falls_inside_it_or_within_the_margin() {
        let r = Reconnects::parse(
            "{\"utc\": \"2026-10-10T19:20:00Z\"}\n{\"utc\": \"2026-10-10T21:00:00Z\"}\n",
        );
        assert_eq!(r.warning, None);
        let during = span("2026-10-10T19:10:00Z", "2026-10-10T19:30:00Z");
        let just_after = span("2026-10-10T20:30:00Z", "2026-10-10T20:59:40Z");
        let well_before = span("2026-10-10T18:00:00Z", "2026-10-10T18:30:00Z");
        let between = span("2026-10-10T19:40:00Z", "2026-10-10T20:20:00Z");
        assert!(r.marks(Some(&during)));
        assert!(r.marks(Some(&just_after)));
        assert!(!r.marks(Some(&well_before)));
        assert!(!r.marks(Some(&between)));
        assert!(!r.marks(None), "a game with no known span is not marked");
    }

    #[test]
    fn bad_lines_are_skipped_with_a_warning_and_the_good_ones_still_mark() {
        let r = Reconnects::parse(
            "not json\n{\"utc\": \"yesterday\"}\n{\"at\": 5}\n\n{\"utc\": \"2026-10-10T19:20:00Z\"}\n",
        );
        assert!(r.warning.as_deref().unwrap().starts_with("3 line(s)"));
        assert!(r.marks(Some(&span("2026-10-10T19:00:00Z", "2026-10-10T19:30:00Z"))));
    }

    #[test]
    fn no_file_means_no_marks_and_no_warning() {
        let dir = temp_dir("none");
        let r = Reconnects::load(&dir);
        assert!(r.is_empty());
        assert_eq!(r.warning, None);
    }

    #[test]
    fn an_unreadable_file_gives_no_marks_and_a_warning() {
        let dir = temp_dir("unreadable");
        // A folder where the file should be cannot be read as a file.
        std::fs::create_dir_all(dir.join(FILE_NAME)).unwrap();
        let r = Reconnects::load(&dir);
        assert!(r.is_empty());
        assert!(r.warning.unwrap().starts_with("Could not read"));
    }

    #[test]
    fn a_file_far_too_big_for_the_tool_is_not_read() {
        let dir = temp_dir("big");
        let f = File::create(dir.join(FILE_NAME)).unwrap();
        f.set_len(MAX_FILE_BYTES + 1).unwrap();
        let r = Reconnects::load(&dir);
        assert!(r.is_empty());
        assert!(r.warning.is_some());
    }

    #[test]
    fn the_file_is_read_from_the_data_folder() {
        let dir = temp_dir("read");
        std::fs::write(dir.join(FILE_NAME), "{\"utc\": \"2026-10-10T19:20:00Z\"}\n").unwrap();
        let r = Reconnects::load(&dir);
        assert!(r.marks(Some(&span("2026-10-10T19:00:00Z", "2026-10-10T19:30:00Z"))));
    }
}
