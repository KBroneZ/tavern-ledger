//! Where each value in the window comes from (T-109). The app shows the
//! label next to the value; it never decides one itself.
//!
//! A value is *from the log* when the game's own log says it, *inferred*
//! when we derive it (tribes seen in the tavern, skin grouping D-019),
//! *entered by you* (nothing yet, ready for T-303), *from the leaderboard*
//! (not in the app yet) or *unknown* when there is nothing to show.

use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Log,
    Inferred,
    Entered,
    Leaderboard,
    #[default]
    Unknown,
}

impl Source {
    pub const ALL: [Source; 5] = [
        Source::Log,
        Source::Inferred,
        Source::Entered,
        Source::Leaderboard,
        Source::Unknown,
    ];

    /// Short word shown next to a value.
    pub fn label(self) -> &'static str {
        match self {
            Source::Log => "from the log",
            Source::Inferred => "inferred",
            Source::Entered => "entered by you",
            Source::Leaderboard => "from the leaderboard",
            Source::Unknown => "unknown",
        }
    }

    /// One sentence for the legend and the tooltip.
    pub fn description(self) -> &'static str {
        match self {
            Source::Log => {
                "The game's own log says so. Counts and averages are worked out from such values."
            }
            Source::Inferred => {
                "Worked out by Tavern Ledger from the log (for example tribes seen in the tavern, or skins grouped under one hero). It can be wrong."
            }
            Source::Entered => "You typed it in.",
            Source::Leaderboard => "Blizzard's public leaderboard says so.",
            Source::Unknown => "Nothing in the log says. It is not zero.",
        }
    }

    /// `Log` when the value exists, `Unknown` when it does not.
    pub fn log_if(known: bool) -> Source {
        if known {
            Source::Log
        } else {
            Source::Unknown
        }
    }
}

/// One line of the legend, with the words from `Source`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LegendEntry {
    pub source: Source,
    pub label: &'static str,
    pub description: &'static str,
}

pub fn legend() -> Vec<LegendEntry> {
    Source::ALL
        .iter()
        .map(|&source| LegendEntry {
            source,
            label: source.label(),
            description: source.description(),
        })
        .collect()
}

/// Source of each value in one ledger row.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RowSources {
    pub played: Source,
    pub mode: Source,
    pub hero: Source,
    pub place: Source,
    pub health: Source,
    pub rounds: Source,
}

/// The session folder name is the log's own, so the date is from the log
/// when it parses as one, unknown when it does not.
fn session_has_date(session: &str) -> bool {
    let Some(rest) = session.strip_prefix("Hearthstone_") else {
        return false;
    };
    let parts: Vec<&str> = rest.split('_').collect();
    let widths = [4, 2, 2, 2, 2, 2];
    parts.len() == widths.len()
        && parts
            .iter()
            .zip(widths)
            .all(|(p, w)| p.len() == w && p.bytes().all(|b| b.is_ascii_digit()))
}

pub fn row_sources(session: &str, report: &Value) -> RowSources {
    let text = |key: &str| report.get(key).is_some_and(Value::is_string);
    let number = |key: &str| report.get(key).is_some_and(Value::is_i64);
    let rounds = report
        .get("rounds")
        .and_then(Value::as_array)
        .is_some_and(|r| !r.is_empty());
    RowSources {
        played: Source::log_if(session_has_date(session)),
        mode: Source::log_if(text("game_type")),
        hero: Source::log_if(text("hero")),
        place: Source::log_if(number("final_place")),
        health: Source::log_if(number("final_health")),
        rounds: Source::log_if(rounds),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SESSION: &str = "Hearthstone_2026_10_09_15_26_25";

    #[test]
    fn every_source_has_a_label_and_a_sentence_for_the_legend() {
        let legend = legend();
        assert_eq!(legend.len(), 5);
        let labels: Vec<_> = legend.iter().map(|e| e.label).collect();
        assert_eq!(
            labels,
            [
                "from the log",
                "inferred",
                "entered by you",
                "from the leaderboard",
                "unknown"
            ]
        );
        assert!(legend.iter().all(|e| !e.description.is_empty()));
    }

    #[test]
    fn sources_serialize_as_snake_case_words() {
        let json = serde_json::to_value(Source::Leaderboard).unwrap();
        assert_eq!(json, json!("leaderboard"));
        assert_eq!(serde_json::to_value(Source::Log).unwrap(), json!("log"));
    }

    #[test]
    fn a_complete_game_is_all_from_the_log() {
        let report = json!({
            "game_type": "GT_BATTLEGROUNDS", "hero": "H", "final_place": 3,
            "final_health": 0, "rounds": [{"number": 1}],
        });
        let s = row_sources(SESSION, &report);
        assert_eq!(
            [s.played, s.mode, s.hero, s.place, s.health, s.rounds],
            [Source::Log; 6]
        );
    }

    #[test]
    fn missing_values_are_unknown_not_from_the_log() {
        let report = json!({"game_type": null, "hero": null, "final_place": null, "rounds": []});
        let s = row_sources(SESSION, &report);
        assert_eq!(
            [s.mode, s.hero, s.place, s.health, s.rounds],
            [Source::Unknown; 5]
        );
    }

    #[test]
    fn a_zero_health_is_a_value_but_a_string_place_is_not() {
        let report = json!({"final_health": 0, "final_place": "3"});
        let s = row_sources(SESSION, &report);
        assert_eq!(s.health, Source::Log);
        assert_eq!(s.place, Source::Unknown);
    }

    #[test]
    fn a_session_name_that_is_not_a_log_folder_has_no_known_date() {
        let report = json!({});
        assert_eq!(row_sources("custom", &report).played, Source::Unknown);
        assert_eq!(
            row_sources("Hearthstone_2026_10_09", &report).played,
            Source::Unknown
        );
        assert_eq!(
            row_sources("Hearthstone_2026_10_09_15_26_2x", &report).played,
            Source::Unknown
        );
        assert_eq!(row_sources(SESSION, &report).played, Source::Log);
    }

    #[test]
    fn a_record_that_is_not_an_object_is_all_unknown() {
        let s = row_sources(SESSION, &json!("x"));
        assert_eq!(s.hero, Source::Unknown);
        assert_eq!(s.place, Source::Unknown);
    }
}
