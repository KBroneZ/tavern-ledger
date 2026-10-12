//! The Duos combats the odds missed (T-609): each one with what the engine
//! does not know about it (approximate or unknown cards) and, when the log
//! has the attacks, the first step where the replay parts from the log. The
//! cause of each is [`crate::why`]'s (D-127), the one the app and the
//! website show; its id is the accuracy export's.
//!
//! Local only, like the rest of the harness: card ids, numbers and rules.

use std::fmt::Write as _;

use std::collections::BTreeMap;

use crate::combat::replay::Rule;
use crate::combat::CombatResult;
use crate::harness::Scored;
use crate::harness_duos::{team_of, DuosCombat};
use crate::harness_logs::Replayed;
use crate::model::Team;
use crate::score::Prediction;
use crate::trials::{simulate_teams, Coverage, Options, Verdict};
use crate::why::{cards_of, not_seen, parted, Cards, Cause, NoReplay, NotSeen};

/// One combat whose most likely result was not the real one.
#[derive(Clone, Debug, PartialEq)]
pub struct Miss {
    pub game: usize,
    pub round: i64,
    pub duos: bool,
    pub prediction: Prediction,
    pub result: CombatResult,
    pub coverage: Coverage,
    /// What the boards bring that the log does not show (D-127).
    pub inputs: Vec<NotSeen>,
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
                inputs: not_seen(&own, &opponent, &[]),
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
            inputs: not_seen(
                &Team::solo(s.combat.player.clone()),
                &Team::solo(s.combat.opponent.clone()),
                &[],
            ),
        })
        .collect()
}

/// Why a miss happened (D-127): from the attack-by-attack replay when the
/// game has one, else only how well the engine knows its cards.
fn find(m: &Miss, replays: &[Replayed]) -> Cause {
    let row = replays
        .iter()
        .find(|r| r.duos == m.duos && r.game == m.game && r.round == m.round);
    match row {
        Some(r) => match &r.replay.difference {
            None => Cause::Chance {
                attacks: r.replay.attacks,
            },
            Some(d) => parted(d.clone(), m.inputs.clone(), &m.coverage),
        },
        None => Cause::NoReplay {
            why: NoReplay::NotSaved,
            cards: cards_of(&m.coverage, false),
        },
    }
}

/// The likely cause of a miss, in a few words.
pub fn cause(m: &Miss, replays: &[Replayed]) -> String {
    match find(m, replays) {
        Cause::Matched => "matched".into(),
        Cause::Chance { .. } => "chance: the replay follows the game to the end".into(),
        Cause::MissingInputs { parted, .. } => format!(
            "something the log does not show; the replay parts at: {}",
            parted.rule.label()
        ),
        Cause::ApproximateCard { parted, .. } => format!(
            "a card not modelled or approximate; the replay parts at: {}",
            parted.rule.label()
        ),
        Cause::RuleDifference(d) => format!("replay parts at: {}", d.rule.label()),
        Cause::NoReplay {
            why: NoReplay::Stops,
            ..
        } => format!("replay parts at: {}", Rule::NoDefender.label()),
        Cause::NoReplay { cards, .. } => match cards {
            Cards::NotModelled => "no replay; a card not modelled (played as a plain minion)",
            Cards::Approximate => "no replay; a card played approximately",
            Cards::Exact => "no replay; every card exact",
        }
        .into(),
    }
}

/// The same cause as a short stable id ([`Cause::id`]), for the accuracy
/// export and the website's words. A new rule or cause needs a new id there.
pub fn cause_id(m: &Miss, replays: &[Replayed]) -> &'static str {
    find(m, replays).id()
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
