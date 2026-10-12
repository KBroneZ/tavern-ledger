//! The Duos combats the odds missed (T-609): each one with what the engine
//! does not know about it (approximate or unknown cards) and, when the log
//! has the attacks, the first step where the replay parts from the log. This
//! is the raw material for the miss breakdown by cause in
//! `docs/research/combat-simulator.md`.
//!
//! Local only, like the rest of the harness: card ids, numbers and rules.

use std::fmt::Write as _;

use crate::combat::CombatResult;
use crate::harness_duos::{team_of, DuosCombat};
use crate::harness_logs::Replayed;
use crate::score::Prediction;
use crate::trials::{simulate_teams, Coverage, Options, Verdict};

/// One combat whose most likely result was not the real one.
#[derive(Clone, Debug, PartialEq)]
pub struct Miss {
    pub game: usize,
    pub round: i64,
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
                prediction,
                result,
                coverage: Coverage::of_teams(&own, &opponent, opts.start_of_combat),
            });
        }
    }
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
                    .find(|r| r.duos && r.game == m.game && r.round == m.round);
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
