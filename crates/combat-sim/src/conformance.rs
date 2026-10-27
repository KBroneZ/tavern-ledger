//! The conformance check (T-642): every corpus combat through the engine,
//! compared with a committed baseline, so a rule change that fixes one card
//! and breaks three others fails with the combats it broke and the attack
//! where each replay now leaves the game.
//!
//! Per combat it records what the engine said (odds, or why not), the most
//! likely result and the real one, and the replay of the saved combat from
//! the first attack to the end, whatever the odds said: the attacks, how
//! many the engine played as the game did, and the first difference (its
//! attack, rule and card). Odds are not recorded: a combat whose two most
//! likely results are within [`CLOSE`] is marked close, and a close combat
//! whose likely result flips is listed apart, never a failure, since a
//! change in how many random numbers the engine draws can flip it.
//!
//! Any other difference from the baseline fails: a regression (a replay
//! that leaves the game earlier or now leaves it, odds lost, a likely
//! result lost), an improvement (the reverse) or a change (another rule or
//! card at the same attack). An improvement fails too, so the baseline is
//! always the engine as it is: rewrite it with `--bless` and commit it with
//! the change, and the diff shows every combat the change moved.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use serde_json::{json, Value};

use crate::combat::replay::Replay;
use crate::combat::CombatResult;
use crate::corpus::CorpusCombat;
use crate::harness::CardDb;
use crate::saved::{explain_game, round_replay, FactsOf, Why};
use crate::score::Prediction;
use crate::trials::Options;
use crate::why::{rule_id, Said};

/// The baseline file's format; a reader refuses any other.
pub const BASELINE_FORMAT: u64 = 1;

/// What the check says about a combat's odds (`Outcome::said`).
pub const SAID: [&str; 5] = [
    "odds",
    "not_modelled",
    "bad_board",
    "no_boards",
    "no_card_data",
];

/// Two likely results closer than this share (of the trials) make a combat
/// close.
pub const CLOSE: f64 = 0.10;

/// Trials per combat: enough for a stable most likely result outside
/// close combats, few enough to keep the check short.
pub const TRIALS: u32 = 2_000;

/// Where the replay first leaves the game.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diverge {
    /// The logged attack (1-based) before or at which the engine parts.
    pub attack: usize,
    /// The rule's id (`replay_attack_order`...), as the accuracy page names
    /// it.
    pub rule: String,
    pub card: Option<String>,
}

/// The engine replayed against the saved combat.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplayCheck {
    pub attacks: usize,
    /// Attacks played as the game did before the first difference.
    pub matched: usize,
    pub diverge: Option<Diverge>,
}

/// One combat's outcome.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub build: u64,
    pub duos: bool,
    /// `odds`, `not_modelled`, `bad_board`, `no_boards` or `no_card_data`.
    pub said: String,
    /// The most likely result when there are odds.
    pub likely: Option<String>,
    pub result: String,
    pub close: bool,
    /// `None` without a readable saved replay or a board.
    pub replay: Option<ReplayCheck>,
}

fn result_word(r: CombatResult) -> &'static str {
    match r {
        CombatResult::Won => "won",
        CombatResult::Tie => "tie",
        CombatResult::Lost => "lost",
    }
}

fn is_close(p: &Prediction) -> bool {
    let mut shares = [p.won, p.tied, p.lost];
    shares.sort_by(|a, b| b.total_cmp(a));
    shares[0] - shares[1] < CLOSE
}

fn replay_check(r: Replay) -> ReplayCheck {
    ReplayCheck {
        attacks: r.attacks,
        matched: r.matched,
        diverge: r.difference.map(|d| Diverge {
            attack: d.attack,
            rule: rule_id(d.rule).to_string(),
            card: d.card,
        }),
    }
}

/// Plays one corpus combat; an error for a line without a combat with a
/// result (the corpus reader lets none through).
pub fn check(c: &CorpusCombat, facts: &FactsOf, options: &Options) -> Result<Outcome, String> {
    let why = explain_game(&c.report, facts, options)
        .into_iter()
        .next()
        .ok_or_else(|| format!("combat {} has no result", c.id))?;
    let (said, likely, close) = match &why.why {
        Why::NoBoards => ("no_boards", None, false),
        Why::NoCardData(_) => ("no_card_data", None, false),
        Why::Explained(e) => match &e.said {
            Said::Odds(p) => ("odds", Some(result_word(p.top())), is_close(p)),
            Said::NotModelled(_) => ("not_modelled", None, false),
            Said::BadBoard => ("bad_board", None, false),
        },
    };
    let replay = round_replay(c.round(), !c.duos, facts, options.seed).map(replay_check);
    Ok(Outcome {
        build: c.build,
        duos: c.duos,
        said: said.to_string(),
        likely: likely.map(String::from),
        result: result_word(why.result).to_string(),
        close,
        replay,
    })
}

/// Every corpus combat's outcome by id, each with the card data of its
/// own build (`cards`); a build without card data is an error.
pub fn run(
    corpus: &[CorpusCombat],
    cards: &BTreeMap<u64, CardDb>,
    options: &Options,
) -> Result<BTreeMap<String, Outcome>, String> {
    let mut out = BTreeMap::new();
    for c in corpus {
        let db = cards
            .get(&c.build)
            .ok_or_else(|| format!("no card data for build {}", c.build))?;
        out.insert(c.id.clone(), check(c, &|id| db.get(id).copied(), options)?);
    }
    Ok(out)
}

fn outcome_json(o: &Outcome) -> Value {
    let replay = o.replay.as_ref().map(|r| {
        let diverge = r
            .diverge
            .as_ref()
            .map(|d| json!({"attack": d.attack, "rule": d.rule, "card": d.card}));
        json!({"attacks": r.attacks, "matched": r.matched, "diverge": diverge})
    });
    json!({
        "build": o.build,
        "mode": if o.duos { "duos" } else { "solo" },
        "said": o.said,
        "likely": o.likely,
        "result": o.result,
        "close": o.close,
        "replay": replay,
    })
}

/// The baseline as pretty JSON, one combat per id in id order, with the
/// options it was made with and the engine's version (for reading only).
pub fn baseline_json(outcomes: &BTreeMap<String, Outcome>, options: &Options) -> String {
    let combats: serde_json::Map<String, Value> = outcomes
        .iter()
        .map(|(id, o)| (id.clone(), outcome_json(o)))
        .collect();
    let v = json!({
        "format": BASELINE_FORMAT,
        "engine": crate::VERSION,
        "trials": options.trials,
        "seed": options.seed,
        "combats": combats,
    });
    // Through the canonical form, so the keys are sorted whatever order
    // serde_json's map keeps.
    let sorted: Value = serde_json::from_str(&crate::corpus::canonical(&v))
        .expect("the canonical form of a value is JSON");
    let mut text = serde_json::to_string_pretty(&sorted).expect("a value prints");
    text.push('\n');
    text
}

fn string(v: &Value, k: &str) -> Result<String, String> {
    v.get(k)
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| format!("{k} is missing"))
}

fn size(v: &Value, k: &str) -> Result<usize, String> {
    v.get(k)
        .and_then(Value::as_u64)
        .and_then(|n| usize::try_from(n).ok())
        .ok_or_else(|| format!("{k} is missing"))
}

fn outcome_from(v: &Value) -> Result<Outcome, String> {
    let replay = match v.get("replay") {
        None | Some(Value::Null) => None,
        Some(r) => Some(ReplayCheck {
            attacks: size(r, "attacks")?,
            matched: size(r, "matched")?,
            diverge: match r.get("diverge") {
                None | Some(Value::Null) => None,
                Some(d) => Some(Diverge {
                    attack: size(d, "attack")?,
                    rule: string(d, "rule")?,
                    card: d.get("card").and_then(Value::as_str).map(String::from),
                }),
            },
        }),
    };
    let duos = match string(v, "mode")?.as_str() {
        "solo" => false,
        "duos" => true,
        _ => return Err("mode is not solo or duos".into()),
    };
    let said = string(v, "said")?;
    if !SAID.contains(&said.as_str()) {
        return Err("said is not one the check writes".into());
    }
    let likely = match v.get("likely") {
        None | Some(Value::Null) => None,
        Some(l) => Some(l.as_str().ok_or("likely is not a result")?.to_string()),
    };
    Ok(Outcome {
        build: v
            .get("build")
            .and_then(Value::as_u64)
            .ok_or("build is missing")?,
        duos,
        said,
        likely,
        result: string(v, "result")?,
        close: v
            .get("close")
            .and_then(Value::as_bool)
            .ok_or("close is missing")?,
        replay,
    })
}

/// A baseline file: its options and outcomes by id.
pub fn read_baseline(text: &str) -> Result<(Options, BTreeMap<String, Outcome>), String> {
    let v: Value = serde_json::from_str(text).map_err(|e| format!("baseline: {e}"))?;
    if v.get("format").and_then(Value::as_u64) != Some(BASELINE_FORMAT) {
        return Err(format!("baseline: format is not {BASELINE_FORMAT}"));
    }
    let trials = v
        .get("trials")
        .and_then(Value::as_u64)
        .and_then(|t| u32::try_from(t).ok())
        .filter(|t| *t > 0)
        .ok_or("baseline: trials is missing or 0")?;
    let seed = v
        .get("seed")
        .and_then(Value::as_u64)
        .ok_or("baseline: seed is missing")?;
    let combats = v
        .get("combats")
        .and_then(Value::as_object)
        .ok_or("baseline: combats is missing")?;
    let outcomes = combats
        .iter()
        .map(|(id, o)| {
            outcome_from(o)
                .map(|o| (id.clone(), o))
                .map_err(|e| format!("baseline {id}: {e}"))
        })
        .collect::<Result<_, _>>()?;
    let options = Options {
        trials,
        seed,
        ..Options::default()
    };
    Ok((options, outcomes))
}

/// How one combat moved.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Regression,
    Improvement,
    Changed,
    /// A close combat whose likely result flipped: listed, not a failure.
    CloseFlip,
}

/// One combat that differs from the baseline.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change {
    pub id: String,
    pub kind: Kind,
    /// What moved, in a few words.
    pub what: String,
}

/// Every combat that differs, the ids only in the baseline (`removed`) and
/// those only in the run (`added`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Diff {
    pub changes: Vec<Change>,
    pub added: Vec<String>,
    pub removed: Vec<String>,
}

impl Diff {
    /// The check fails on any difference but a close flip.
    pub fn fails(&self) -> bool {
        !self.added.is_empty()
            || !self.removed.is_empty()
            || self.changes.iter().any(|c| c.kind != Kind::CloseFlip)
    }
}

fn step(r: &Option<ReplayCheck>) -> String {
    match r {
        None => "no replay".into(),
        Some(ReplayCheck {
            diverge: None,
            attacks,
            ..
        }) => format!("follows all {attacks} attacks"),
        Some(ReplayCheck {
            diverge: Some(d), ..
        }) => format!(
            "leaves at attack {} ({}{})",
            d.attack,
            d.rule,
            d.card.as_ref().map_or(String::new(), |c| format!(", {c}"))
        ),
    }
}

/// How a replay moved: `None` when it did not.
fn replay_kind(old: &Option<ReplayCheck>, new: &Option<ReplayCheck>) -> Option<Kind> {
    if old == new {
        return None;
    }
    let point = |r: &ReplayCheck| r.diverge.as_ref().map(|d| d.attack);
    Some(match (old, new) {
        (Some(o), Some(n)) => match (point(o), point(n)) {
            (Some(_), None) => Kind::Improvement,
            (None, Some(_)) => Kind::Regression,
            (Some(a), Some(b)) if b > a => Kind::Improvement,
            (Some(a), Some(b)) if b < a => Kind::Regression,
            _ => Kind::Changed,
        },
        (Some(o), None) if o.diverge.is_none() => Kind::Regression,
        (None, Some(n)) if n.diverge.is_none() => Kind::Improvement,
        _ => Kind::Changed,
    })
}

/// How one combat moved, if it did: the replay first, then the odds
/// verdict, then the likely result; every part that moved is named.
fn change(old: &Outcome, new: &Outcome) -> Option<(Kind, String)> {
    let mut parts: Vec<(Kind, String)> = Vec::new();
    if let Some(kind) = replay_kind(&old.replay, &new.replay) {
        parts.push((
            kind,
            format!("{} -> {}", step(&old.replay), step(&new.replay)),
        ));
    }
    if old.said != new.said {
        let kind = match (old.said.as_str(), new.said.as_str()) {
            ("odds", _) => Kind::Regression,
            (_, "odds") => Kind::Improvement,
            _ => Kind::Changed,
        };
        parts.push((kind, format!("said {} -> {}", old.said, new.said)));
    } else if old.likely != new.likely || old.result != new.result {
        let right = |o: &Outcome| o.likely.as_deref() == Some(o.result.as_str());
        // Only a combat close before and after may flip without failing: one
        // that was clear and became close and wrong did move.
        let kind = if old.close && new.close {
            Kind::CloseFlip
        } else {
            match (right(old), right(new)) {
                (true, false) => Kind::Regression,
                (false, true) => Kind::Improvement,
                _ => Kind::Changed,
            }
        };
        let word = |o: &Outcome| o.likely.clone().unwrap_or_else(|| "none".into());
        parts.push((
            kind,
            format!(
                "likely {} -> {} (result {})",
                word(old),
                word(new),
                new.result
            ),
        ));
    }
    // Only the close mark moved: nothing a reader acts on.
    let first = parts.first()?.0;
    let kind = if parts.iter().all(|(k, _)| *k == first) {
        first
    } else if parts.iter().any(|(k, _)| *k == Kind::Regression) {
        Kind::Regression
    } else {
        Kind::Changed
    };
    let what = parts
        .into_iter()
        .map(|(_, w)| w)
        .collect::<Vec<_>>()
        .join("; ");
    Some((kind, what))
}

/// The run against the baseline.
pub fn compare(baseline: &BTreeMap<String, Outcome>, now: &BTreeMap<String, Outcome>) -> Diff {
    let mut diff = Diff::default();
    for (id, old) in baseline {
        match now.get(id) {
            None => diff.removed.push(id.clone()),
            Some(new) => {
                if let Some((kind, what)) = change(old, new) {
                    diff.changes.push(Change {
                        id: id.clone(),
                        kind,
                        what,
                    });
                }
            }
        }
    }
    diff.added = now
        .keys()
        .filter(|id| !baseline.contains_key(*id))
        .cloned()
        .collect();
    diff.changes
        .sort_by(|a, b| (a.kind, &a.id).cmp(&(b.kind, &b.id)));
    diff
}

/// The diff, grouped by kind, one combat per line.
pub fn render_diff(diff: &Diff) -> String {
    let mut out = String::new();
    let label = |k: Kind| match k {
        Kind::Regression => "regression",
        Kind::Improvement => "improvement",
        Kind::Changed => "changed",
        Kind::CloseFlip => "close flip (not a failure)",
    };
    for c in &diff.changes {
        let _ = writeln!(out, "{}: {} {}", label(c.kind), c.id, c.what);
    }
    for id in &diff.added {
        let _ = writeln!(out, "new in the corpus: {id} (not in the baseline)");
    }
    for id in &diff.removed {
        let _ = writeln!(out, "gone from the corpus: {id} (still in the baseline)");
    }
    out
}

/// Counts over a run, per mode: likely result right, odds given, replays
/// that follow the game to the end, and where the others part, by rule.
pub fn render_summary(outcomes: &BTreeMap<String, Outcome>) -> String {
    let mut out = String::new();
    for (duos, mode) in [(false, "Solo"), (true, "Duos")] {
        let all: Vec<&Outcome> = outcomes.values().filter(|o| o.duos == duos).collect();
        if all.is_empty() {
            continue;
        }
        let odds: Vec<&&Outcome> = all.iter().filter(|o| o.said == "odds").collect();
        let right = odds
            .iter()
            .filter(|o| o.likely.as_deref() == Some(o.result.as_str()))
            .count();
        let replays: Vec<&ReplayCheck> = all.iter().filter_map(|o| o.replay.as_ref()).collect();
        let follow = replays.iter().filter(|r| r.diverge.is_none()).count();
        let pct = |a: usize, b: usize| {
            if b == 0 {
                0.0
            } else {
                100.0 * a as f64 / b as f64
            }
        };
        let _ = writeln!(out, "{mode}: {} combats", all.len());
        let _ = writeln!(
            out,
            "  odds given: {} of {} ({:.1}%); likely result right: {right} of {} ({:.1}%)",
            odds.len(),
            all.len(),
            pct(odds.len(), all.len()),
            odds.len(),
            pct(right, odds.len())
        );
        let _ = writeln!(
            out,
            "  replays that follow the game to the end: {follow} of {} ({:.1}%)",
            replays.len(),
            pct(follow, replays.len())
        );
        let mut rules: BTreeMap<&str, usize> = BTreeMap::new();
        for d in replays.iter().filter_map(|r| r.diverge.as_ref()) {
            *rules.entry(d.rule.as_str()).or_default() += 1;
        }
        for (rule, n) in rules {
            let _ = writeln!(out, "    first difference {rule}: {n}");
        }
    }
    out
}

#[cfg(test)]
#[path = "conformance_tests.rs"]
mod tests;
