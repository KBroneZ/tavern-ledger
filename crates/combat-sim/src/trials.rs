//! Many random combats between the same two warbands: the odds to win, tie or
//! lose and how much damage each result deals.

use std::collections::BTreeMap;

use crate::cards::{self, Action, Fidelity, Trigger};
use crate::combat::{play, CombatResult, Prepared};
use crate::model::{BoardError, Keywords, Tribes, Warband};
use crate::rng::Rng;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    pub trials: u32,
    pub seed: u64,
    /// Play start-of-combat effects. False for a board taken after them (the
    /// logged boards are taken at the first attack).
    pub start_of_combat: bool,
    /// Play cards the engine does not know as plain minions and return odds
    /// marked "not modelled" instead of refusing. For measuring the engine
    /// only, never for odds shown to a player.
    pub allow_unmodelled: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            trials: 10_000,
            seed: 0x5EED,
            start_of_combat: false,
            allow_unmodelled: false,
        }
    }
}

/// Which cards on the two boards the engine cannot play exactly.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Coverage {
    /// Card ids with no rules, or with a start-of-combat effect the engine
    /// does not play when start of combat is on.
    pub not_modelled: Vec<String>,
    /// Card ids played with a known gap, and the gap.
    pub approximate: Vec<(String, &'static str)>,
}

impl Coverage {
    pub fn of(player: &Warband, opponent: &Warband, start_of_combat: bool) -> Coverage {
        let mut cov = Coverage::default();
        for m in player.minions.iter().chain(&opponent.minions) {
            let id = &m.card_id;
            match cards::rules(id) {
                None => push_unique(&mut cov.not_modelled, id.clone()),
                Some(rules) => {
                    let soc_gap = start_of_combat
                        && rules.triggers.contains(&Trigger::StartOfCombatUnmodelled);
                    if soc_gap {
                        push_unique(&mut cov.not_modelled, id.clone());
                    }
                    if let Fidelity::Approximate(why) = rules.fidelity {
                        if !cov.approximate.iter().any(|(c, _)| c == id) {
                            cov.approximate.push((id.clone(), why));
                        }
                    }
                }
            }
        }
        for band in [player, opponent] {
            let grants = reborn_grants(band);
            for m in &band.minions {
                let may_reborn = m.keywords.contains(Keywords::REBORN)
                    || grants.iter().any(|t| *t == Tribes::NONE || m.tribes.is(*t));
                if may_reborn
                    && m.base.is_none()
                    && !cov.approximate.iter().any(|(c, _)| *c == m.card_id)
                {
                    cov.approximate.push((
                        m.card_id.clone(),
                        "may come back with Reborn; card stats unknown",
                    ));
                }
            }
        }
        cov
    }

    pub fn is_exact(&self) -> bool {
        self.not_modelled.is_empty() && self.approximate.is_empty()
    }
}

/// Tribes a card on this warband can give Reborn to in combat.
fn reborn_grants(band: &Warband) -> Vec<Tribes> {
    let mut out = Vec::new();
    for m in &band.minions {
        let Some(rules) = cards::rules(&m.card_id) else {
            continue;
        };
        for t in rules.triggers {
            let actions: &[Action] = match t {
                Trigger::Deathrattle(a)
                | Trigger::Rally(a)
                | Trigger::Avenge(_, a)
                | Trigger::StartOfCombat(a) => a,
                Trigger::FriendlyAttacks { actions, .. } => actions,
                _ => &[],
            };
            for a in actions {
                if let Action::GiveKeyword { keyword, tribe, .. } = a {
                    if keyword.contains(Keywords::REBORN) {
                        out.push(*tribe);
                    }
                }
            }
        }
    }
    out
}

fn push_unique(list: &mut Vec<String>, id: String) {
    if !list.contains(&id) {
        list.push(id);
    }
}

/// The spread of results over all trials, for the player.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Odds {
    pub trials: u32,
    pub won: u32,
    pub tied: u32,
    pub lost: u32,
    /// Damage dealt to the opponent's hero -> trials, over the won trials
    /// where every tier is known.
    pub damage_dealt: BTreeMap<i32, u32>,
    /// Damage taken -> trials, over the lost trials where every tier is known.
    pub damage_taken: BTreeMap<i32, u32>,
    /// Won or lost trials whose damage is unknown (a tier was missing).
    pub damage_unknown: u32,
    /// Trials that hit the turn limit (counted as ties).
    pub stalled: u32,
    pub coverage: Coverage,
}

impl Odds {
    pub fn share(&self, result: CombatResult) -> f64 {
        let n = match result {
            CombatResult::Won => self.won,
            CombatResult::Tie => self.tied,
            CombatResult::Lost => self.lost,
        };
        f64::from(n) / f64::from(self.trials.max(1))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Odds the engine can give (check `coverage` for approximations).
    Odds(Odds),
    /// A card on the boards is not modelled: no number, by design.
    NotModelled(Coverage),
}

/// Plays `options.trials` combats between the player's warband and the
/// opponent's. Deterministic for a given seed.
pub fn simulate(
    player: &Warband,
    opponent: &Warband,
    options: &Options,
) -> Result<Verdict, BoardError> {
    player.validate()?;
    opponent.validate()?;
    if options.trials == 0 {
        return Err(BoardError::NoTrials);
    }
    let coverage = Coverage::of(player, opponent, options.start_of_combat);
    if !coverage.not_modelled.is_empty() && !options.allow_unmodelled {
        return Ok(Verdict::NotModelled(coverage));
    }
    let prepared = Prepared::new(player, opponent, options.start_of_combat);
    let mut rng = Rng::new(options.seed);
    let mut odds = Odds {
        trials: options.trials,
        won: 0,
        tied: 0,
        lost: 0,
        damage_dealt: BTreeMap::new(),
        damage_taken: BTreeMap::new(),
        damage_unknown: 0,
        stalled: 0,
        coverage,
    };
    for _ in 0..options.trials {
        let outcome = play(&prepared, &mut rng);
        if outcome.stalled {
            odds.stalled += 1;
        }
        let spread = match outcome.result {
            CombatResult::Won => {
                odds.won += 1;
                &mut odds.damage_dealt
            }
            CombatResult::Lost => {
                odds.lost += 1;
                &mut odds.damage_taken
            }
            CombatResult::Tie => {
                odds.tied += 1;
                continue;
            }
        };
        match outcome.damage {
            Some(d) => *spread.entry(d).or_insert(0) += 1,
            None => odds.damage_unknown += 1,
        }
    }
    Ok(Verdict::Odds(odds))
}
