//! The Duos combats the odds missed (T-609): each one with what the engine
//! does not know about it (approximate or unknown cards) and, when the log
//! has the attacks, the first step where the replay parts from the log. This
//! is the raw material for the miss breakdown by cause in
//! the design notes.
//!
//! Local only, like the rest of the harness: card ids, numbers and rules.

use std::fmt::Write as _;

use std::collections::BTreeMap;

use crate::combat::replay::Rule;
use crate::combat::CombatResult;
use crate::harness::Scored;
use crate::harness_duos::{team_of, DuosCombat};
use crate::harness_logs::Replayed;
use crate::score::Prediction;
use crate::trials::{simulate_teams, Coverage, Options, Verdict};

/// One combat whose most likely result was not the real one.
#[derive(Clone, Debug, PartialEq)]
pub struct Miss {
    pub game: usize,
    pub round: i64,
    pub duos: bool,
    pub prediction: Prediction,
    pub result: CombatResult,
    pub coverage: Coverage,
}

/// The Duos combats with a result whose most likely result was wrong,
/// played as in [`crate::harness_duos::render_duos`] (unknown cards as
/// plain minions).
pub fn duos_misses(combats: &[DuosCombat], options: &Options) -> Vec<Miss> {
    let opts = Options {
        allow_unmodelled: true,
        ..*options
    };
    let mut out = Vec::new();
    for c in combats {
        let Some(result) = c.result else { continue };
        let (own, opponent) = (team_of(&c.own), team_of(&c.opponent));
        let Ok(Verdict::Odds(o)) = simulate_teams(&own, &opponent, &opts) else {
            continue;
        };
        let prediction = Prediction {
            won: o.share(CombatResult::Won),
            tied: o.share(CombatResult::Tie),
            lost: o.share(CombatResult::Lost),
        };
        if prediction.top() != result {
            out.push(Miss {
                game: c.game,
                round: c.round,
                duos: true,
                prediction,
                result,
                coverage: Coverage::of_teams(&own, &opponent, opts.start_of_combat),
            });
        }
    }
    out
}

/// The Solo combats of a run whose most likely result was wrong.
pub fn solo_misses(scored: &[Scored]) -> Vec<Miss> {
    scored
        .iter()
        .filter(|s| s.prediction.top() != s.combat.result)
        .map(|s| Miss {
            game: s.combat.game,
            round: s.combat.round,
            duos: false,
            prediction: s.prediction,
            result: s.combat.result,
            coverage: s.coverage.clone(),
        })
        .collect()
}

/// Where a miss most likely comes from, found from the attack-by-attack
/// replay when the game has one, else from the cards.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Found {
    /// The replay follows the game to the end: the dice.
    Chance,
    /// The first rule where the replay parts from the log.
    Replay(Rule),
    NotModelled,
    Approximate,
    Exact,
}

fn find(m: &Miss, replays: &[Replayed]) -> Found {
    let row = replays
        .iter()
        .find(|r| r.duos == m.duos && r.game == m.game && r.round == m.round);
    match row.map(|r| &r.replay.difference) {
        Some(None) => Found::Chance,
        Some(Some(d)) => Found::Replay(d.rule),
        None if !m.coverage.not_modelled.is_empty() => Found::NotModelled,
        None if !m.coverage.approximate.is_empty() => Found::Approximate,
        None => Found::Exact,
    }
}

/// The likely cause of a miss, in a few words: from the attack-by-attack
/// replay when the game has one (the first rule that differs, or chance
/// when the replay follows the game to the end), else from the cards.
pub fn cause(m: &Miss, replays: &[Replayed]) -> String {
    match find(m, replays) {
        Found::Chance => "chance: the replay follows the game to the end".into(),
        Found::Replay(rule) => format!("replay parts at: {}", rule.label()),
        Found::NotModelled => "no replay; a card not modelled (played as a plain minion)".into(),
        Found::Approximate => "no replay; a card played approximately".into(),
        Found::Exact => "no replay; every card exact".into(),
    }
}

/// The same cause as a short stable id, for the accuracy export and the
/// website's words. A new rule or cause needs a new id there.
pub fn cause_id(m: &Miss, replays: &[Replayed]) -> &'static str {
    match find(m, replays) {
        Found::Chance => "chance",
        Found::Replay(Rule::FirstAttacker) => "replay_first_attacker",
        Found::Replay(Rule::AfterTeammate) => "replay_after_teammate",
        Found::Replay(Rule::AttackOrder) => "replay_attack_order",
        Found::Replay(Rule::Target) => "replay_target",
        Found::Replay(Rule::Minions) => "replay_minions",
        Found::Replay(Rule::Stats) => "replay_stats",
        Found::Replay(Rule::End) => "replay_end",
        Found::Replay(Rule::NoDefender) => "replay_no_defender",
        Found::NotModelled => "no_replay_not_modelled",
        Found::Approximate => "no_replay_approximate",
        Found::Exact => "no_replay_exact",
    }
}

/// Misses counted by cause: cause -> (Solo, Duos).
pub fn causes(misses: &[Miss], replays: &[Replayed]) -> BTreeMap<String, (u32, u32)> {
    let mut out: BTreeMap<String, (u32, u32)> = BTreeMap::new();
    for m in misses {
        let slot = out.entry(cause(m, replays)).or_insert((0, 0));
        if m.duos {
            slot.1 += 1;
        } else {
            slot.0 += 1;
        }
    }
    out
}

/// Misses counted by cause id: (id, Solo, Duos), most first, then by id.
pub fn causes_by_id(misses: &[Miss], replays: &[Replayed]) -> Vec<(&'static str, u32, u32)> {
    let mut counts: BTreeMap<&'static str, (u32, u32)> = BTreeMap::new();
    for m in misses {
        let slot = counts.entry(cause_id(m, replays)).or_insert((0, 0));
        if m.duos {
            slot.1 += 1;
        } else {
            slot.0 += 1;
        }
    }
    let mut out: Vec<_> = counts.into_iter().map(|(id, (s, d))| (id, s, d)).collect();
    out.sort_by(|a, b| (b.1 + b.2).cmp(&(a.1 + a.2)).then(a.0.cmp(b.0)));
    out
}

fn pct(x: f64) -> String {
    format!("{:.0}%", x * 100.0)
}

/// The misses, one line each; `replays` adds the replay's first difference
/// for combats the logs have the attacks of.
pub fn render_misses(misses: &[Miss], replays: Option<&[Replayed]>) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "\n### Duos combats the odds missed (T-609)\n");
    if misses.is_empty() {
        let _ = writeln!(out, "None.");
        return out;
    }
    for m in misses {
        let mut cards: Vec<&str> = m.coverage.not_modelled.iter().map(String::as_str).collect();
        cards.extend(m.coverage.approximate.iter().map(|(c, _)| c.as_str()));
        let cards = if cards.is_empty() {
            "every card exact".to_string()
        } else {
            format!("not exact: {}", cards.join(", "))
        };
        let replay = match replays {
            None => String::new(),
            Some(rows) => {
                let row = rows
                    .iter()
                    .find(|r| r.duos == m.duos && r.game == m.game && r.round == m.round);
                match row.map(|r| (&r.replay.difference, r.replay.attacks)) {
                    None => "; replay: no attacks in the log".into(),
                    Some((None, _)) => "; replay: follows the log to the end (chance)".into(),
                    Some((Some(d), n)) => format!(
                        "; replay: attack {} of {n}, {}; {}",
                        d.attack,
                        d.rule.label(),
                        d.detail
                    ),
                }
            }
        };
        let _ = writeln!(
            out,
            "- game {} round {}: real {:?}, said won {} / tie {} / lost {}; {cards}{replay}",
            m.game,
            m.round,
            m.result,
            pct(m.prediction.won),
            pct(m.prediction.tied),
            pct(m.prediction.lost),
        );
    }
    out
}

#[cfg(test)]
#[path = "harness_misses_tests.rs"]
mod tests;
