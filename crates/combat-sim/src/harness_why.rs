//! Why each result differed from its odds, counted over a set of saved
//! games (T-346, D-127): every combat with a result through
//! [`crate::saved::explain_game`], the same function the app's recap and
//! the website use. Counts, rules and card ids only: no game or round.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use serde_json::Value;

use crate::harness::CardDb;
use crate::saved::{explain_game, Why};
use crate::trials::Options;
use crate::why::{Cause, NoReplay, Said, Unseen};

/// Combats counted by a label: (Solo, Duos).
pub type Counts = BTreeMap<String, (u32, u32)>;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tally {
    /// By cause key (`matched`, `chance`...) or why there are no odds.
    pub causes: Counts,
    /// Within each cause: what was not seen, which card, which rule.
    pub details: Counts,
}

fn add(counts: &mut Counts, label: String, duos: bool) {
    let slot = counts.entry(label).or_insert((0, 0));
    if duos {
        slot.1 += 1;
    } else {
        slot.0 += 1;
    }
}

fn unseen_word(u: Unseen) -> &'static str {
    match u {
        Unseen::BloodGems => "Blood Gem upgrades",
        Unseen::Spells => "Tavern spell upgrades",
        Unseen::Knights => "Eternal Knights that died",
        Unseen::Beetles => "Beetle buffs",
        Unseen::Hand => "a minion from the hand",
        Unseen::Earlier => "gains from earlier in the game",
        Unseen::Gift => "a Dark Gift the log does not name",
    }
}

fn no_replay_word(n: NoReplay) -> &'static str {
    match n {
        NoReplay::NotSaved => "none saved",
        NoReplay::Unreadable => "unreadable",
        NoReplay::Stops => "stops early",
    }
}

/// The labels one combat counts under: its cause and its details.
fn labels(why: &Why) -> (String, Vec<String>) {
    let e = match why {
        Why::NoBoards => return ("no odds: a board not seen".into(), Vec::new()),
        Why::NoCardData(_) => return ("no odds: no card data".into(), Vec::new()),
        Why::Explained(e) => e,
    };
    let cause = match (&e.said, &e.cause) {
        (Said::NotModelled(ids), _) => {
            let details = ids
                .iter()
                .map(|id| format!("not modelled: `{id}`"))
                .collect();
            return ("no odds: a card not modelled".into(), details);
        }
        (_, Some(c)) => c,
        _ => return ("no odds: boards refused".into(), Vec::new()),
    };
    let details = match cause {
        Cause::MissingInputs { inputs, .. } => {
            let mut words: Vec<String> = inputs
                .iter()
                .map(|n| format!("not seen: {}", unseen_word(n.what)))
                .collect();
            words.dedup();
            words
        }
        Cause::ApproximateCard { cards, .. } => cards
            .iter()
            .map(|id| format!("approximate: `{id}`"))
            .collect(),
        Cause::RuleDifference(d) => d.card.iter().map(|c| format!("at: `{c}`")).collect(),
        Cause::NoReplay { why, .. } => vec![format!("no replay: {}", no_replay_word(*why))],
        Cause::Matched | Cause::Chance { .. } => Vec::new(),
    };
    (cause.id().into(), details)
}

/// Every combat with a result of every saved game, by cause.
pub fn tally(reports: &[Value], db: &CardDb, options: &Options) -> Tally {
    let facts = |id: &str| db.get(id).copied();
    let mut t = Tally::default();
    for report in reports {
        let duos = report.get("game_type").and_then(Value::as_str) == Some("GT_BATTLEGROUNDS_DUO");
        for c in explain_game(report, &facts, options) {
            let (cause, details) = labels(&c.why);
            add(&mut t.causes, cause, duos);
            for d in details {
                add(&mut t.details, d, duos);
            }
        }
    }
    t
}

pub fn render_why(t: &Tally) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "\n## Why the result differed (D-127)\n\nEvery combat with a result in the saved \
         games, by the cause the app and the website show.\n"
    );
    let _ = writeln!(out, "| Cause | Solo | Duos |");
    let _ = writeln!(out, "|---|---|---|");
    for (label, (solo, duos)) in &t.causes {
        let _ = writeln!(out, "| {label} | {solo} | {duos} |");
    }
    if !t.details.is_empty() {
        let _ = writeln!(out, "\n| Detail | Solo | Duos |");
        let _ = writeln!(out, "|---|---|---|");
        for (label, (solo, duos)) in &t.details {
            let _ = writeln!(out, "| {label} | {solo} | {duos} |");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::load_cards;

    const CARDS: &str = include_str!("../tests/data/cards.json");
    const GAMES: &str = include_str!("../tests/data/games.jsonl");

    #[test]
    fn every_combat_with_a_result_is_counted_once_by_cause() {
        let db = load_cards(CARDS).unwrap();
        let reports: Vec<Value> = GAMES
            .lines()
            .filter_map(|l| serde_json::from_str::<Value>(l).ok())
            .filter_map(|v| v.get("report").cloned())
            .collect();
        let options = Options {
            trials: 100,
            ..Options::default()
        };
        let t = tally(&reports, &db, &options);
        let counted: u32 = t.causes.values().map(|(s, d)| s + d).sum();
        let with_result: usize = reports
            .iter()
            .filter(|r| {
                let mode = r.get("game_type").and_then(Value::as_str);
                matches!(mode, Some("GT_BATTLEGROUNDS" | "GT_BATTLEGROUNDS_DUO"))
            })
            .flat_map(|r| r["rounds"].as_array().cloned().unwrap_or_default())
            .filter(|r| crate::saved::result(r).is_some() && r.get("number").is_some())
            .count();
        assert_eq!(counted as usize, with_result);
        let text = render_why(&t);
        assert!(text.contains("| Cause | Solo | Duos |"), "{text}");
    }
}
