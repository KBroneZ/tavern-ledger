//! The counterfactual engine's fights (T-657): each pair of boards played
//! by the simulator in batches, the actual board and an alternative in
//! step, until the difference between them is clear, the trials run out or
//! the time does. Every board's trials are kept for the whole game, so the
//! actual board of a turn is played once for all of its decisions.
//!
//! Batch `k` of a board always uses the seed `SEED ^ k`, so a board's odds
//! after `k` batches are the same whatever order the decisions came in, and
//! the actual board and an alternative share their seeds batch by batch.
//!
//! The time: every decision gets a slice of what is left (the time left
//! over the decisions left), so the first turns cannot use it all. Every
//! board still gets its first batch while the game's time lasts; only the
//! batches after it stop at the end of the slice.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use combat_sim::{simulate_teams, Odds, Options, Team, Verdict};
use serde::Serialize;

use super::Budget;

/// Fixed, so the same boards give the same odds.
const SEED: u64 = 0xC0F7;
/// A difference is clear beyond this many standard errors. A game makes a
/// few hundred comparisons with a few looks each; at 4 the chance of
/// calling chance a difference is about 6 in 100 000 per look, so a game
/// rarely has one.
pub(super) const Z: f64 = 4.0;
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
    /// The boards (or the hero's kit) are not ones the simulator accepts.
    BadBoard,
    /// The time budget ran out, or the review was cancelled, before this
    /// board was played.
    OutOfTime,
}

/// Why a comparison stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Stop {
    /// The difference is beyond four standard errors: one board is clearly
    /// better.
    Clear,
    /// Four standard errors are below 2 points of score: whatever the
    /// difference is, it is surely small. In practice only fights whose
    /// result hardly varies stop here.
    Close,
    /// Every trial the budget allows was played; the difference may still
    /// be unclear.
    MaxTrials,
    /// The decision's share of the time, or the game's, ran out first.
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

/// The standard error of the difference of two boards' scores, taking
/// their trials as independent: the shared seeds hardly tie them once the
/// boards differ, so this is about right.
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

/// Whether the difference between two boards is beyond [`Z`] standard
/// errors.
pub(super) fn clear(a: &Odds, b: &Odds) -> bool {
    (score(b) - score(a)).abs() > Z * standard_error(a, b)
}

/// Whether the difference between two boards is clear, or surely small.
fn settled(a: &Odds, b: &Odds) -> Option<Stop> {
    if clear(a, b) {
        Some(Stop::Clear)
    } else if Z * standard_error(a, b) < CLOSE {
        Some(Stop::Close)
    } else {
        None
    }
}

/// A pair of teams: the player's, then the opponent's.
pub(super) type Pair = (Team, Team);

/// One board's trials so far.
struct Run {
    odds: Odds,
    batches: u64,
}

/// One pair of boards played this game.
struct Board {
    teams: Pair,
    run: Result<Option<Run>, Gap>,
}

/// Every board played this game, found again by its teams.
pub(super) struct Sims<'c> {
    boards: Vec<Board>,
    budget: Budget,
    elapsed: &'c mut dyn FnMut() -> Duration,
    cancel: Option<&'c AtomicBool>,
    /// When the current decision's share of the time ends.
    slice_end: Duration,
    /// The game's time ran out, or the review was cancelled; every board
    /// not played yet is [`Gap::OutOfTime`].
    pub out_of_time: bool,
    pub cancelled: bool,
    pub trials: u64,
}

/// A comparison's result: both boards' odds as it stopped, and why.
pub(super) struct Compared {
    pub actual: Odds,
    pub alternative: Result<Odds, Gap>,
    pub stop: Stop,
}

impl<'c> Sims<'c> {
    pub(super) fn new(
        budget: Budget,
        elapsed: &'c mut dyn FnMut() -> Duration,
        cancel: Option<&'c AtomicBool>,
    ) -> Self {
        Sims {
            boards: Vec::new(),
            budget,
            elapsed,
            cancel,
            slice_end: budget.time,
            out_of_time: false,
            cancelled: false,
            trials: 0,
        }
    }

    /// Boards played so far.
    pub(super) fn boards(&self) -> usize {
        self.boards.len()
    }

    /// Gives the next decision its share of the time left, `left` decisions
    /// to go (this one among them).
    pub(super) fn start_decision(&mut self, left: usize) {
        if self.out_of_time {
            return;
        }
        let now = (self.elapsed)();
        let rest = self.budget.time.saturating_sub(now);
        let share = rest / u32::try_from(left.max(1)).unwrap_or(u32::MAX);
        self.slice_end = now + share;
    }

    /// The time it is now, after marking the game out of time when it is.
    fn now(&mut self) -> Duration {
        if self.cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
            self.cancelled = true;
            self.out_of_time = true;
        }
        let now = (self.elapsed)();
        if now >= self.budget.time {
            self.out_of_time = true;
        }
        now
    }

    /// Whether a board's first batch may still run.
    fn game_over(&mut self) -> bool {
        self.out_of_time || {
            self.now();
            self.out_of_time
        }
    }

    /// Whether a further batch may still run for this decision.
    fn slice_over(&mut self) -> bool {
        if self.out_of_time {
            return true;
        }
        let now = self.now();
        self.out_of_time || now >= self.slice_end
    }

    /// The index of a pair of boards, added when new.
    fn slot(&mut self, teams: &Pair) -> usize {
        match self.boards.iter().position(|b| b.teams == *teams) {
            Some(i) => i,
            None => {
                self.boards.push(Board {
                    teams: teams.clone(),
                    run: Ok(None),
                });
                self.boards.len() - 1
            }
        }
    }

    /// Trials so far; None when the board cannot be played.
    fn trials_of(&self, i: usize) -> Option<u32> {
        match &self.boards[i].run {
            Ok(r) => Some(r.as_ref().map_or(0, |r| r.odds.trials)),
            Err(_) => None,
        }
    }

    /// Plays one more batch of a board, unless it is at the most trials.
    fn batch(&mut self, i: usize) {
        let max = self.budget.max_trials.max(1);
        let (done, batches) = match &self.boards[i].run {
            Ok(r) => r.as_ref().map_or((0, 0), |r| (r.odds.trials, r.batches)),
            Err(_) => return,
        };
        if done >= max {
            return;
        }
        let trials = self.budget.batch.max(1).min(max - done);
        let options = Options {
            trials,
            seed: SEED ^ batches,
            start_of_combat: false,
            allow_unmodelled: false,
        };
        let teams = &self.boards[i].teams;
        let outcome = match simulate_teams(&teams.0, &teams.1, &options) {
            Ok(Verdict::Odds(o)) => Ok(o),
            Ok(Verdict::NotModelled(_)) => Err(Gap::NotModelled),
            Err(_) => Err(Gap::BadBoard),
        };
        self.trials += u64::from(trials);
        let board = &mut self.boards[i];
        board.run = match (outcome, board.run.as_mut()) {
            (Ok(o), Ok(Some(run))) => {
                run.odds.merge(&o);
                run.batches += 1;
                return;
            }
            (Ok(o), _) => Ok(Some(Run {
                odds: o,
                batches: 1,
            })),
            (Err(g), _) => Err(g),
        };
    }

    fn odds_at(&self, i: usize) -> Result<Odds, Gap> {
        match &self.boards[i].run {
            Ok(Some(r)) => Ok(r.odds.clone()),
            Ok(None) => Err(Gap::OutOfTime),
            Err(g) => Err(*g),
        }
    }

    /// A board's odds so far, or why it has none.
    pub(super) fn odds(&mut self, teams: &Pair) -> Result<Odds, Gap> {
        let i = self.slot(teams);
        self.odds_at(i)
    }

    /// Plays the actual board's first batch, unless it has one or the
    /// game's time is up.
    pub(super) fn play(&mut self, actual: &Pair) -> Result<Odds, Gap> {
        let i = self.slot(actual);
        if self.trials_of(i) == Some(0) && !self.game_over() {
            self.batch(i);
        }
        self.odds_at(i)
    }

    /// Plays the actual board and an alternative in step, the one with
    /// fewer trials first, until their difference is settled, both are at
    /// the most trials, or the time is up (checked before every batch).
    /// The actual board has been played ([`Sims::play`]).
    pub(super) fn compare(&mut self, actual: &Pair, alt: &Pair) -> Result<Compared, Gap> {
        let (ia, ib) = (self.slot(actual), self.slot(alt));
        let max = self.budget.max_trials.max(1);
        self.odds_at(ia)?;
        let stop = loop {
            let na = self.trials_of(ia).unwrap_or(0);
            let Some(nb) = self.trials_of(ib) else {
                break Stop::Gap;
            };
            if nb == 0 {
                if self.game_over() {
                    break Stop::OutOfTime;
                }
                self.batch(ib);
                continue;
            }
            if let (Ok(Some(a)), Ok(Some(b))) = (&self.boards[ia].run, &self.boards[ib].run) {
                if let Some(s) = settled(&a.odds, &b.odds) {
                    break s;
                }
            }
            if na >= max && nb >= max {
                break Stop::MaxTrials;
            }
            if self.slice_over() {
                break Stop::OutOfTime;
            }
            self.batch(if na <= nb && na < max { ia } else { ib });
        };
        Ok(Compared {
            actual: self.odds_at(ia)?,
            alternative: self.odds_at(ib),
            stop,
        })
    }
}
