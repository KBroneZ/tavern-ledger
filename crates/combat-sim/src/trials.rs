//! Many random combats between the same two warbands: the odds to win, tie or
//! lose and how much damage each result deals.

use std::collections::BTreeMap;

use crate::cards::{self, Action, Fidelity, Trigger};
use crate::combat::{play, CombatResult, Prepared};
use crate::gifts;
use crate::model::{BoardError, GameValues, Keywords, Minion, Team, Tribes, Warband};
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
    /// Card ids (and Dark Gift ids) with no rules, or with a start-of-combat
    /// effect the engine does not play when start of combat is on.
    pub not_modelled: Vec<String>,
    /// Card ids (and Dark Gift ids) played with a known gap, and the gap.
    pub approximate: Vec<(String, &'static str)>,
}

impl Coverage {
    pub fn of(player: &Warband, opponent: &Warband, start_of_combat: bool) -> Coverage {
        Coverage::of_boards(&[player, opponent], start_of_combat)
    }

    /// Every board of both teams (Duos).
    pub fn of_teams(player: &Team, opponent: &Team, start_of_combat: bool) -> Coverage {
        let boards: Vec<&Warband> = player.boards().chain(opponent.boards()).collect();
        Coverage::of_boards(&boards, start_of_combat)
    }

    fn of_boards(boards: &[&Warband], start_of_combat: bool) -> Coverage {
        let mut cov = Coverage::default();
        let minions = boards
            .iter()
            .flat_map(|b| b.minions.iter().map(|m| (m, &b.values)));
        for (m, values) in minions {
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
                        if !played_with(why, values)
                            && !cov.approximate.iter().any(|(c, _)| c == id)
                        {
                            cov.approximate.push((id.clone(), why));
                        }
                    }
                }
            }
        }
        for m in boards.iter().flat_map(|b| &b.minions) {
            cov.add_gifts(m, start_of_combat);
        }
        for band in boards {
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

    /// A minion's Dark Gifts, by the same rules as cards: a gift the engine
    /// does not know (or whose start-of-combat effect it does not play, when
    /// start of combat is on) blocks; an approximate one is named.
    fn add_gifts(&mut self, m: &Minion, start_of_combat: bool) {
        for id in &m.gifts {
            let Some(gift) = gifts::gift(id) else {
                push_unique(&mut self.not_modelled, id.clone());
                continue;
            };
            if start_of_combat && gift.triggers.contains(&Trigger::StartOfCombatUnmodelled) {
                push_unique(&mut self.not_modelled, id.clone());
            }
            if let Fidelity::Approximate(why) = gifts::fidelity(gift, &m.card_id) {
                if !self.approximate.iter().any(|(c, _)| c == id) {
                    self.approximate.push((id.clone(), why));
                }
            }
        }
    }

    pub fn is_exact(&self) -> bool {
        self.not_modelled.is_empty() && self.approximate.is_empty()
    }

    /// Why each card of [`Coverage::not_modelled`] blocks the odds, in the
    /// same order, for the page to show (T-616).
    pub fn reasons(&self) -> Vec<(String, &'static str)> {
        self.not_modelled
            .iter()
            .map(|id| (id.clone(), refusal_reason(id)))
            .collect()
    }
}

/// Why the engine refuses a card or Dark Gift: a start-of-combat effect it
/// does not play on a board taken before it, or the reason written for the
/// card (T-616).
pub fn refusal_reason(id: &str) -> &'static str {
    if gifts::gift(id).is_some() || cards::rules(id).is_some() {
        return START_OF_COMBAT_GAP;
    }
    if id.starts_with("BG36_MidGameEffect") {
        return GIFT_NOT_REVIEWED;
    }
    cards::refusal(id)
}

const START_OF_COMBAT_GAP: &str =
    "its Start of Combat effect is not played, and this board was taken before it";
const GIFT_NOT_REVIEWED: &str = "the simulator has not reviewed this Dark Gift yet";

/// True when the card's only gap is a game-long value its board brings
/// (T-610): Blood Gem or Tavern spell upgrades above zero. Zero is unknown
/// (a board without values), so the card stays approximate then.
fn played_with(why: &str, values: &GameValues) -> bool {
    match why {
        w if w == crate::cards_round1::BLOOD_GEM => values.blood_gem != (0, 0),
        w if w == crate::cards_round2::SPELL => values.spell != (0, 0),
        _ => false,
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
            for a in t.actions() {
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
    /// Adds another run over the same boards (another seed): the counts and
    /// spreads add up.
    pub fn merge(&mut self, other: &Odds) {
        self.trials += other.trials;
        self.won += other.won;
        self.tied += other.tied;
        self.lost += other.lost;
        for (mine, theirs) in [
            (&mut self.damage_dealt, &other.damage_dealt),
            (&mut self.damage_taken, &other.damage_taken),
        ] {
            for (d, n) in theirs {
                *mine.entry(*d).or_insert(0) += n;
            }
        }
        self.damage_unknown += other.damage_unknown;
        self.stalled += other.stalled;
    }

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
    Ok(Verdict::Odds(run_trials(&prepared, coverage, options)))
}

/// A Duos combat (T-608): each team's first board fights; once one is gone
/// its teammate's board comes in, and the combat ends when a team has no
/// board left. The damage is the tier of the hero whose board is on the
/// field at the end plus the tiers of every surviving minion of the team,
/// a teammate's board that never came in included. A team without a second
/// board plays as in Solo. Deterministic for a given seed.
pub fn simulate_teams(
    player: &Team,
    opponent: &Team,
    options: &Options,
) -> Result<Verdict, BoardError> {
    player.validate()?;
    opponent.validate()?;
    if options.trials == 0 {
        return Err(BoardError::NoTrials);
    }
    let coverage = Coverage::of_teams(player, opponent, options.start_of_combat);
    if !coverage.not_modelled.is_empty() && !options.allow_unmodelled {
        return Ok(Verdict::NotModelled(coverage));
    }
    let prepared = Prepared::teams(player, opponent, options.start_of_combat);
    Ok(Verdict::Odds(run_trials(&prepared, coverage, options)))
}

fn run_trials(prepared: &Prepared, coverage: Coverage, options: &Options) -> Odds {
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
        let outcome = play(prepared, &mut rng);
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
    odds
}
