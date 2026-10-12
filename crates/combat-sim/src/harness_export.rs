//! What the app would show on the user's combats (T-604, T-607): how many
//! combats a card not modelled blocks, by card, and the accuracy of the odds
//! on the combats it does not block, per mode. The same numbers go to a
//! small JSON file for the website (`--export`); its format is in
//! the design notes.
//!
//! The JSON holds counts, shares, the date, the engine version and card ids
//! only: no names, seats, heroes, games or rounds.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use serde_json::{json, Value};

use crate::cards;
use crate::combat::CombatResult;
use crate::harness::LoggedCombat;
use crate::harness_duos::{team_of, DuosCombat};
use crate::harness_logs::Replayed;
use crate::harness_misses::{causes_by_id, Miss};
use crate::model::{BoardError, Warband};
use crate::score::{score, Prediction, Report};
use crate::trials::{simulate, simulate_teams, Odds, Options, Verdict};

/// Version of the JSON's shape; a reader checks it before reading.
pub const FORMAT: u32 = 3;

/// The cards that summon a minion from the hand: approximate since round 5
/// (D-097), counted apart.
pub use crate::cards::HAND_SUMMONERS;

/// Calibration buckets in the export and the report.
const BUCKETS: usize = 5;

/// Card ids listed in the export, most blocking first.
const EXPORT_BLOCKERS: usize = 10;

/// Game builds listed in the export, newest first.
const EXPORT_BUILDS: usize = 20;

/// One mode's combats with a result: blocked, refused, and the score of the
/// rest.
#[derive(Clone, Debug, PartialEq)]
pub struct ModeAccuracy {
    pub with_result: u32,
    /// A card on a board is not modelled: the app shows no number.
    pub blocked: u32,
    /// A hand summoner on a board (blocked or not).
    pub with_hand_summoner: u32,
    /// Refused by the engine's board check.
    pub refused: u32,
    pub report: Report,
    /// The scored combats with a hand summoner on a board (approximate:
    /// the hand summon is left out, D-097). Not in the export.
    pub hand_scored: Report,
    /// The blocked combats played anyway, the cards not modelled as plain
    /// minions: evidence for keeping a card blocking. Not in the export.
    pub anyway: Report,
}

impl ModeAccuracy {
    pub fn blocked_share(&self) -> f64 {
        f64::from(self.blocked) / f64::from(self.with_result.max(1))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Accuracy {
    pub solo: ModeAccuracy,
    pub duos: ModeAccuracy,
    /// Card id -> combats it blocks (both modes), most first, then by id.
    pub blockers: Vec<(String, u32)>,
}

/// One combat with a result: the card ids on its boards the engine does not
/// model (each once), and whether a hand summoner is on a board.
struct Item {
    result: CombatResult,
    blocking: Vec<String>,
    hand: bool,
}

fn item<'a>(result: CombatResult, boards: impl IntoIterator<Item = &'a Warband>) -> Item {
    let ids: Vec<&str> = boards
        .into_iter()
        .flat_map(|b| &b.minions)
        .map(|m| m.card_id.as_str())
        .collect();
    let mut blocking: Vec<String> = ids
        .iter()
        .filter(|id| cards::rules(id).is_none())
        .map(|id| id.to_string())
        .collect();
    blocking.sort();
    blocking.dedup();
    Item {
        result,
        blocking,
        hand: ids.iter().any(|id| HAND_SUMMONERS.contains(id)),
    }
}

fn prediction(o: &Odds) -> Prediction {
    Prediction {
        won: o.share(CombatResult::Won),
        tied: o.share(CombatResult::Tie),
        lost: o.share(CombatResult::Lost),
    }
}

/// Scores one mode. `play(i, anyway)` plays combat `i`; `anyway` plays the
/// cards not modelled as plain minions (only for a blocked combat, scored
/// apart).
fn mode(
    combats: &[Item],
    mut play: impl FnMut(usize, bool) -> Option<Odds>,
    blockers: &mut BTreeMap<String, u32>,
) -> ModeAccuracy {
    let mut out = ModeAccuracy {
        with_result: 0,
        blocked: 0,
        with_hand_summoner: 0,
        refused: 0,
        report: score(&[], BUCKETS),
        hand_scored: score(&[], BUCKETS),
        anyway: score(&[], BUCKETS),
    };
    let (mut items, mut hand, mut anyway) = (Vec::new(), Vec::new(), Vec::new());
    for (i, c) in combats.iter().enumerate() {
        out.with_result += 1;
        out.with_hand_summoner += u32::from(c.hand);
        if !c.blocking.is_empty() {
            out.blocked += 1;
            for id in &c.blocking {
                *blockers.entry(id.clone()).or_insert(0) += 1;
            }
            anyway.extend(play(i, true).map(|o| (prediction(&o), c.result)));
            continue;
        }
        match play(i, false) {
            Some(o) => {
                let scored = (prediction(&o), c.result);
                if c.hand {
                    hand.push(scored);
                }
                items.push(scored);
            }
            None => out.refused += 1,
        }
    }
    out.report = score(&items, BUCKETS);
    out.hand_scored = score(&hand, BUCKETS);
    out.anyway = score(&anyway, BUCKETS);
    out
}

fn odds(v: Result<Verdict, BoardError>) -> Option<Odds> {
    match v {
        Ok(Verdict::Odds(o)) => Some(o),
        _ => None,
    }
}

/// Plays every combat with a result as the app would: cards not modelled
/// refused, start of combat off (the logged boards are taken after it).
pub fn accuracy(solo: &[LoggedCombat], duos: &[DuosCombat], options: &Options) -> Accuracy {
    let opts = Options {
        allow_unmodelled: false,
        start_of_combat: false,
        ..*options
    };
    let anyway = Options {
        allow_unmodelled: true,
        ..opts
    };
    let pick = |a: bool| if a { &anyway } else { &opts };
    let mut blockers = BTreeMap::new();
    let solo_items: Vec<_> = solo
        .iter()
        .map(|c| item(c.result, [&c.player, &c.opponent]))
        .collect();
    let solo_acc = mode(
        &solo_items,
        |i, a| odds(simulate(&solo[i].player, &solo[i].opponent, pick(a))),
        &mut blockers,
    );
    let duos: Vec<&DuosCombat> = duos.iter().filter(|c| c.result.is_some()).collect();
    let duos_items: Vec<_> = duos
        .iter()
        .filter_map(|c| {
            let boards = c.own.iter().chain(&c.opponent).map(|l| &l.board);
            Some(item(c.result?, boards))
        })
        .collect();
    let duos_acc = mode(
        &duos_items,
        |i, a| {
            let c = duos[i];
            odds(simulate_teams(
                &team_of(&c.own),
                &team_of(&c.opponent),
                pick(a),
            ))
        },
        &mut blockers,
    );
    let mut blockers: Vec<(String, u32)> = blockers.into_iter().collect();
    blockers.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    Accuracy {
        solo: solo_acc,
        duos: duos_acc,
        blockers,
    }
}

fn pct(x: f64) -> String {
    format!("{:.0}%", x * 100.0)
}

/// The report section: blocked combats by card, then the accuracy of the
/// rest, per mode.
pub fn render_accuracy(a: &Accuracy) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "\n## What the app would show (T-604, T-607)\n\nEvery combat with a result, on its \
         own boards, as the app plays them: a card not modelled blocks the odds.\n"
    );
    let _ = writeln!(
        out,
        "| Mode | With a result | Blocked | With a hand summoner | Refused | Scored | \
         Mean p(real result) | Brier | Most likely = real |"
    );
    let _ = writeln!(out, "|---|---|---|---|---|---|---|---|---|");
    for (label, m) in [("Solo", &a.solo), ("Duos", &a.duos)] {
        let blocked = if m.with_result > 0 {
            format!("{} ({})", m.blocked, pct(m.blocked_share()))
        } else {
            "-".into()
        };
        let _ = writeln!(
            out,
            "| {label} | {} | {blocked} | {} | {} | {} | {} |",
            m.with_result,
            m.with_hand_summoner,
            m.refused,
            m.report.combats,
            scores(&m.report),
        );
    }
    let _ = writeln!(out, "\nCards that block a combat (card id: combats):\n");
    if a.blockers.is_empty() {
        let _ = writeln!(out, "None.");
    } else {
        let list: Vec<String> = a
            .blockers
            .iter()
            .map(|(id, n)| format!("`{id}` {n}"))
            .collect();
        let _ = writeln!(out, "{}", list.join(", "));
    }
    render_anyway(&mut out, a);
    out
}

/// Two parts of the combats, scored apart: the scored ones with a hand
/// summoner, and the blocked ones played anyway.
fn render_anyway(out: &mut String, a: &Accuracy) {
    let _ = writeln!(
        out,
        "\nTwo parts of these combats, apart: the scored ones with a hand summoner ({}), \
         approximate since round 5 (D-097: the hand summon is left out), and the blocked ones \
         played anyway, the cards not modelled as plain minions (the app shows no number for \
         these).\n\n\
         | Mode | Combats | n | Mean p(real result) | Brier | Most likely = real |\n\
         |---|---|---|---|---|---|",
        HAND_SUMMONERS.join(", ")
    );
    for (label, m) in [("Solo", &a.solo), ("Duos", &a.duos)] {
        let groups = [
            ("scored, with a hand summoner", &m.hand_scored),
            ("blocked, played anyway", &m.anyway),
        ];
        for (group, r) in groups {
            let _ = writeln!(out, "| {label} | {group} | {} | {} |", r.combats, scores(r));
        }
    }
}

/// Mean p(real result), Brier and most-likely-right as table cells.
fn scores(r: &Report) -> String {
    if r.combats == 0 {
        return "- | - | -".into();
    }
    format!(
        "{} | {:.3} | {} of {}",
        pct(r.mean_p_actual),
        r.brier,
        r.top_pick_hits,
        r.combats
    )
}

fn mode_json(m: &ModeAccuracy) -> Value {
    let r = &m.report;
    let some = |count: u32, x: f64| if count > 0 { json!(x) } else { Value::Null };
    let buckets: Vec<Value> = r
        .buckets
        .iter()
        .map(|b| {
            json!({
                "from": b.lo,
                "to": b.hi,
                "predictions": b.count,
                "mean_said": some(b.count, b.mean_predicted),
                "happened": some(b.count, b.observed),
            })
        })
        .collect();
    json!({
        "combats_with_result": m.with_result,
        "blocked": m.blocked,
        "blocked_share": some(m.with_result, m.blocked_share()),
        "refused": m.refused,
        "scored": r.combats,
        "mean_p_real": some(r.combats, r.mean_p_actual),
        "brier": some(r.combats, r.brier),
        "most_likely_right": r.top_pick_hits,
        "calibration": buckets,
    })
}

/// What was checked besides the odds (T-553): the misses and the replays
/// that tell their cause, and the game builds of the games read (build ->
/// games).
pub struct Checked<'a> {
    pub misses: &'a [Miss],
    pub replays: &'a [Replayed],
    pub builds: &'a BTreeMap<i64, u32>,
}

fn misses_json(c: &Checked) -> Value {
    let total = c.misses.len();
    let causes: Vec<Value> = causes_by_id(c.misses, c.replays)
        .into_iter()
        .map(|(id, solo, duos)| {
            json!({
                "cause": id,
                "solo": solo,
                "duos": duos,
                "share": f64::from(solo + duos) / total as f64,
            })
        })
        .collect();
    json!({ "total": total, "causes": causes })
}

fn builds_json(builds: &BTreeMap<i64, u32>) -> Value {
    let checked: Vec<Value> = builds
        .iter()
        .rev()
        .take(EXPORT_BUILDS)
        .map(|(build, games)| json!({ "build": build, "count": games }))
        .collect();
    json!({
        "newest": builds.keys().next_back(),
        "checked": checked,
    })
}

/// The export (T-607, T-553): counts and shares per mode, the date
/// (`YYYY-MM-DD`), the engine version, the card ids that block most, the
/// misses by cause and the game builds checked (newest first). Pretty-printed,
/// ending in a new line.
pub fn export_json(a: &Accuracy, options: &Options, date: &str, checked: &Checked) -> String {
    let blockers: Vec<Value> = a
        .blockers
        .iter()
        .take(EXPORT_BLOCKERS)
        .map(|(id, n)| json!({ "card_id": id, "combats": n }))
        .collect();
    let v = json!({
        "format": FORMAT,
        "date": date,
        "engine_version": env!("CARGO_PKG_VERSION"),
        "trials": options.trials,
        "modes": {
            "solo": mode_json(&a.solo),
            "duos": mode_json(&a.duos),
        },
        "blocking_cards": blockers,
        "misses": misses_json(checked),
        "builds": builds_json(checked.builds),
    });
    let mut s = serde_json::to_string_pretty(&v).unwrap_or_default();
    s.push('\n');
    s
}

/// `YYYY-MM-DD` of a day counted from 1970-01-01 (UTC), from the civil
/// calendar's own rules (no dependency).
pub fn date_of_day(days: i64) -> String {
    // Shift to 0000-03-01 so a leap day ends the year.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

#[cfg(test)]
#[path = "harness_export_tests.rs"]
mod tests;
