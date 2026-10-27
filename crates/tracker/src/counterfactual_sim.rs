//! The counterfactual engine's fights (T-657): each pair of boards played
//! by the simulator in batches, the actual board and an alternative in
//! step, until the difference between them is clear, the trials run out or
//! the time does. Every board's trials are kept for the whole game, so the
//! actual board of a turn is played once for all of its decisions.
//!
//! Batch `k` of a board always uses the seed `SEED ^ k`, so a board's odds
//! after `k` batches are the same whatever order the decisions came in, and
//! the actual board and an alternative share their seeds batch by batch
//! (the same dice for both, which makes their difference steadier).

use std::collections::BTreeMap;
use std::time::Duration;

use combat_sim::{simulate_teams, Odds, Options, Team, Verdict};
use serde::Serialize;

use super::Budget;

/// Fixed, so the same boards give the same odds.
const SEED: u64 = 0xC0F7;
/// A difference is clear beyond this many standard errors. With at most a
/// few looks per pair the chance of calling chance a difference stays near
/// 2 in 100.
pub(super) const Z: f64 = 3.0;
/// Two boards are as good as each other when the difference is surely
/// within this share of the score.
const CLOSE: f64 = 0.02;

/// Why a board has no fight to show.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Gap {
    /// The card data is not on the PC, or lacks a card on the boards.
    NoCardData,
    /// The card offered is not a minion (a spell): nothing to put on the board.
    NotAMinion,
    /// The card offered acts at the start of combat, which the boards saved
    /// at the first attack can no longer play.
    StartOfCombat,
    /// A card on the boards is not in the simulator.
    NotModelled,
    /// The boards are not ones the simulator accepts.
    BadBoard,
    /// The time budget ran out before this board was played.
    OutOfTime,
}

/// Why a comparison stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Stop {
    /// One board is clearly better.
    Clear,
    /// The two are surely within a small margin of each other.
    Close,
    /// Every trial the budget allows was played; the difference may still
    /// be unclear.
    MaxTrials,
    /// The time budget ran out first.
    OutOfTime,
    /// A board could not be played.
    Gap,
}

/// The score of a fight: a win counts 1, a tie a half.
pub(super) fn score(o: &Odds) -> f64 {
    if o.trials == 0 {
        return 0.0;
    }
    (f64::from(o.won) + 0.5 * f64::from(o.tied)) / f64::from(o.trials)
}

/// The variance of one trial's score.
fn variance(o: &Odds) -> f64 {
    if o.trials == 0 {
        return 0.0;
    }
    let s = score(o);
    let square = (f64::from(o.won) + 0.25 * f64::from(o.tied)) / f64::from(o.trials);
    (square - s * s).max(0.0)
}

/// The standard error of the difference of two boards' scores, as if their
/// trials were independent (the shared seeds make it, if anything, smaller).
pub(super) fn standard_error(a: &Odds, b: &Odds) -> f64 {
    let part = |o: &Odds| {
        if o.trials == 0 {
            0.0
        } else {
            variance(o) / f64::from(o.trials)
        }
    };
    (part(a) + part(b)).sqrt()
}

/// Whether the difference between two boards is clear, or surely small.
fn settled(a: &Odds, b: &Odds) -> Option<Stop> {
    let d = (score(b) - score(a)).abs();
    let margin = Z * standard_error(a, b);
    if d > margin {
        Some(Stop::Clear)
    } else if margin < CLOSE {
        Some(Stop::Close)
    } else {
        None
    }
}

/// One board's trials so far.
struct Run {
    odds: Odds,
    batches: u64,
}

/// Every board played this game, by its teams.
pub(super) struct Sims<'c> {
    cache: BTreeMap<String, Result<Run, Gap>>,
    budget: Budget,
    elapsed: &'c mut dyn FnMut() -> Duration,
    /// The time budget ran out; every board after it is [`Gap::OutOfTime`].
    pub out_of_time: bool,
    pub trials: u64,
}

/// A pair of boards' odds after a comparison.
pub(super) struct Compared {
    pub alternative: Result<Odds, Gap>,
    pub stop: Stop,
}

/// The cache key of a pair of teams: their full form, in this run only.
fn key(teams: &(Team, Team)) -> String {
    format!("{:?}", teams)
}

impl<'c> Sims<'c> {
    pub(super) fn new(budget: Budget, elapsed: &'c mut dyn FnMut() -> Duration) -> Self {
        Sims {
            cache: BTreeMap::new(),
            budget,
            elapsed,
            out_of_time: false,
            trials: 0,
        }
    }

    /// Boards played so far.
    pub(super) fn boards(&self) -> usize {
        self.cache.len()
    }

    fn over_time(&mut self) -> bool {
        if !self.out_of_time && (self.elapsed)() >= self.budget.time {
            self.out_of_time = true;
        }
        self.out_of_time
    }

    fn trials_of(&self, k: &str) -> Option<u32> {
        match self.cache.get(k) {
            None => Some(0),
            Some(Ok(r)) => Some(r.odds.trials),
            Some(Err(_)) => None,
        }
    }

    /// Plays one more batch of a board, unless it is at the most trials.
    fn batch(&mut self, k: &str, teams: &(Team, Team)) {
        let done = self.trials_of(k).unwrap_or(u32::MAX);
        let max = self.budget.max_trials.max(1);
        if done >= max {
            return;
        }
        let batches = match self.cache.get(k) {
            Some(Ok(r)) => r.batches,
            _ => 0,
        };
        let trials = self.budget.batch.max(1).min(max - done);
        let options = Options {
            trials,
            seed: SEED ^ batches,
            start_of_combat: false,
            allow_unmodelled: false,
        };
        let outcome = match simulate_teams(&teams.0, &teams.1, &options) {
            Ok(Verdict::Odds(o)) => Ok(o),
            Ok(Verdict::NotModelled(_)) => Err(Gap::NotModelled),
            Err(_) => Err(Gap::BadBoard),
        };
        self.trials += u64::from(trials);
        match (outcome, self.cache.get_mut(k)) {
            (Ok(o), Some(Ok(run))) => {
                run.odds.merge(&o);
                run.batches += 1;
            }
            (Ok(o), _) => {
                self.cache.insert(
                    k.to_string(),
                    Ok(Run {
                        odds: o,
                        batches: 1,
                    }),
                );
            }
            (Err(g), _) => {
                self.cache.insert(k.to_string(), Err(g));
            }
        }
    }

    /// A board's odds so far, or why it has none.
    pub(super) fn odds(&self, teams: &(Team, Team)) -> Result<Odds, Gap> {
        match self.cache.get(&key(teams)) {
            Some(Ok(r)) => Ok(r.odds.clone()),
            Some(Err(g)) => Err(*g),
            None => Err(Gap::OutOfTime),
        }
    }

    /// Plays the actual board alone (at least one batch), for a decision
    /// whose alternatives all fail before a fight.
    pub(super) fn play(&mut self, actual: &(Team, Team)) -> Result<Odds, Gap> {
        let k = key(actual);
        if self.trials_of(&k) == Some(0) && !self.over_time() {
            self.batch(&k, actual);
        }
        self.odds(actual)
    }

    /// Plays the actual board and an alternative in step, the one with
    /// fewer trials first, until their difference is settled, both are at
    /// the most trials, or the time is up (checked before every batch).
    pub(super) fn compare(&mut self, actual: &(Team, Team), alt: &(Team, Team)) -> Compared {
        let (ka, kb) = (key(actual), key(alt));
        let max = self.budget.max_trials.max(1);
        let stop = loop {
            let (Some(na), Some(nb)) = (self.trials_of(&ka), self.trials_of(&kb)) else {
                break Stop::Gap;
            };
            if na > 0 && nb > 0 {
                if let (Ok(a), Ok(b)) = (self.odds(actual), self.odds(alt)) {
                    if let Some(s) = settled(&a, &b) {
                        break s;
                    }
                }
                if na >= max && nb >= max {
                    break Stop::MaxTrials;
                }
            }
            if self.over_time() {
                break Stop::OutOfTime;
            }
            if na <= nb && na < max {
                self.batch(&ka, actual);
            } else {
                self.batch(&kb, alt);
            }
        };
        Compared {
            alternative: self.odds(alt),
            stop,
        }
    }
}
