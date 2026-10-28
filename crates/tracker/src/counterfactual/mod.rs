//! The counterfactual engine (T-657), for the game review (Phase 4): for
//! each decision of a saved Solo game ([`crate::decisions`]), what the
//! next combat would have looked like had the player taken another card on
//! offer, or kept the minion sold. Post-game only (D-176): it reads a saved
//! game and never a game being played.
//!
//! What is rebuilt, on the boards the record saved for the combat after
//! the decision (shop turn `t` is followed by combat `t`) and the one
//! before it (combat `t - 1`, or none before the game's first):
//! - a buy or a choice of a minion: the minion taken must be new on that
//!   board (one copy, none on the board of the combat before), and each
//!   other card on offer takes its place, at its card data's stats;
//! - a choice of a trinket: each other trinket on offer takes its place in
//!   the hero's kit;
//! - a sale: the minion sold (the only copy on the board of the combat
//!   before, none after) is put back where it was, with its stats then;
//!   when a minion was bought after the sale that turn, in place of the
//!   first such buy, the sale's gold having paid for it (`sale.rs`).
//!
//! Everything else says why it is not rebuilt ([`NotRebuilt`]), never
//! dropped: a roll, freeze, tier-up or spell would need shops and gold the
//! log never showed. Only the next combat is played: what an alternative
//! would have changed in later turns is not known. What an alternative
//! takes as given is listed with it ([`Approx`]): a Battlecry not played,
//! gold not counted, stats from the card data.
//!
//! The actual board and each alternative go through the same steps: both
//! are played by the simulator (`sim.rs`), never compared with the real
//! result, from the boards at the first attack (the record has no other),
//! so start of combat is not played. A card that acts at the start of
//! combat cannot be swapped in or out ([`Gap::StartOfCombat`],
//! [`NotRebuilt::StartOfCombat`]); one elsewhere on the boards is noted
//! ([`Approx::StartOfCombatOnBoard`]). Both heroes' tavern tiers are left
//! out of the damage (the record lacks the opponent's, and it is the same
//! either way): the damage given is the tiers of the minions left standing.
//!
//! With a model of expected place, each board also gets the place the model
//! expects for the position entering that combat with that board
//! ([`Place`]); without one, or with one players may not see yet, a marker
//! and never a number. That place does not use the simulated fight yet:
//! the model reads both boards entering it.
//!
//! The whole game runs within a time budget ([`Budget`]), shared out over
//! the decisions; a board the time did not reach says so
//! ([`Gap::OutOfTime`]).

use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

use combat_sim::{CombatResult, Odds};
use expected_place::model::{places, Model, INPUTS};
use serde::Serialize;
use serde_json::Value;

use crate::decisions::{decisions, Decision, DecisionKind};
use crate::odds::Facts;
use crate::shop::ShopState;

mod game;
mod sale;
mod sim;

pub use sim::{Gap, Stop};

use game::Game;
use sim::{clear, score, standard_error, Sims};

/// How much the engine may do for one game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Budget {
    /// For the whole game, checked before every batch of trials. Each
    /// decision gets its share of what is left.
    pub time: Duration,
    /// The most trials of one board.
    pub max_trials: u32,
    /// Trials per batch; whether a comparison is settled is checked
    /// between batches.
    pub batch: u32,
}

impl Default for Budget {
    /// Well within the minute the review has once a game ends.
    fn default() -> Self {
        Budget {
            time: Duration::from_secs(40),
            max_trials: 4_000,
            batch: 500,
        }
    }
}

/// Why a game gets no review.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NoReview {
    /// Not a Solo Battlegrounds game (Duos comes later).
    NotSolo,
    /// The record has no shop to read decisions from.
    NoShop(ShopState),
}

/// Why a decision is not rebuilt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NotRebuilt {
    /// A roll, freeze, unfreeze, tier-up or spell: the alternative needs
    /// shops and gold the log never showed.
    NeedsShopPlayout,
    /// The record does not say which card was taken or sold.
    NoCardTaken,
    /// Nothing else was on offer.
    NoAlternative,
    /// The combat after the decision has no board on a side: the game
    /// ended, or the combat was not shown.
    NoNextCombat,
    /// The combat before the decision has no board of the player's, so
    /// what is new on the board is not known.
    NoBoardBefore,
    /// The card taken is neither on the next combat's board nor among the
    /// hero's trinkets (in hand, sold, a spell, made golden...).
    TakenNotOnBoard,
    /// The minion sold is not on the board of the combat before.
    SoldNotOnBoard,
    /// Copies of the card on the boards leave open which one it was.
    Ambiguous,
    /// Keeping the minion sold would need an eighth place on the board.
    BoardFull,
    /// The minion bought after the sale, which keeping the one sold would
    /// have skipped, is not on the next combat's board (in hand, made
    /// golden...), or the log does not name it.
    NextBuyNotOnBoard,
    /// The sale or a buy of its turn has no time in the log: whether the
    /// sale's gold paid for a buy is not known.
    OrderUnknown,
    /// The card taken or sold acts at the start of combat, which the saved
    /// boards already show or can no longer play.
    StartOfCombat,
    /// The actual board could not be played (for a trinket, the hero's kit
    /// could not be read).
    Actual(Gap),
}

/// What an alternative takes as given.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Approx {
    /// The card put in has its card data's stats, without what the minion
    /// it replaces gained after it was taken.
    BaseStats,
    /// Neither card's Battlecry, nor anything else done as it was played,
    /// is played again: what the card taken did stays on the board, what
    /// the other would have done is not there.
    BattlecryNotPlayed,
    /// The gold each card would have cost, and left for the rest of the
    /// turn, is not counted.
    GoldIgnored,
    /// The minion kept has its stats from the combat before it was sold.
    EarlierStats,
    /// A card on the boards acts at the start of combat: it might have
    /// changed the card put in too.
    StartOfCombatOnBoard,
    /// The minion is kept although gold was spent after its sale that turn
    /// (a roll, a tier-up, a spell) or the shop frozen: the sale's gold may
    /// have paid for that, so keeping it may not have been possible.
    GoldMaybeShort,
}

/// The place the model expects for a board, or why there is no number.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Place {
    /// No model was given.
    NoModel,
    /// Trained on fewer games than players may see numbers from.
    TooFewGames,
    /// A Duos model.
    OtherMode,
    /// Trained on rows of another format, or not of the expected shape.
    OtherFormat,
    /// The record gives no row for the combat.
    NoRow,
    /// The expected place, 1 the best.
    Expected(f64),
}

impl Place {
    fn number(self) -> Option<f64> {
        match self {
            Place::Expected(p) => Some(p),
            _ => None,
        }
    }
}

/// One board's next combat as the simulator played it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Fight {
    pub won: f64,
    pub tied: f64,
    pub lost: f64,
    pub trials: u32,
    /// Trials that hit the turn limit, counted as ties.
    pub stalled: u32,
    /// The tiers of the player's minions left standing, on average over
    /// every trial (0 when not won); None when a tier is unknown.
    pub minion_damage_dealt: Option<f64>,
    /// The tiers of the opponent's minions left standing, likewise.
    pub minion_damage_taken: Option<f64>,
    pub place: Place,
    /// Card ids the simulator plays with a known gap, and those of minions
    /// gifted with no gift named.
    pub approximate: Vec<String>,
}

impl Fight {
    /// A win counts 1, a tie a half.
    pub fn score(&self) -> f64 {
        self.won + 0.5 * self.tied
    }
}

/// An alternative against the actual board, from the trials both had when
/// their comparison stopped (the actual board may have more by the end of
/// the decision): each number is the alternative's minus the actual one's.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Delta {
    /// The next combat's score (a win 1, a tie a half): above 0, the
    /// alternative did better in it. Not the cost in final place.
    pub score: f64,
    pub standard_error: f64,
    /// The difference is beyond four standard errors; true exactly when
    /// the comparison stopped as [`Stop::Clear`].
    pub clear: bool,
    /// The actual board's trials it was measured against.
    pub actual_trials: u32,
    pub minion_damage_dealt: Option<f64>,
    pub minion_damage_taken: Option<f64>,
    /// Expected place: below 0, the alternative's is better. None unless
    /// both boards have a number.
    pub place: Option<f64>,
}

/// One card the player could have taken instead, or the minion kept.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Alternative {
    pub card: String,
    /// For a sale: the card id of the minion bought after it that turn,
    /// which the alternative does not buy (the sale paid for it).
    pub instead_of: Option<String>,
    pub approx: Vec<Approx>,
    pub fight: Result<Fight, Gap>,
    pub delta: Option<Delta>,
    /// Why its comparison stopped; None when it was never played.
    pub stop: Option<Stop>,
}

/// A decision played again: the actual board's next combat and each
/// alternative's.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Rebuilt {
    /// The combat played.
    pub round: i64,
    /// The actual board with every trial it had by the decision's end.
    pub actual: Fight,
    pub alternatives: Vec<Alternative>,
    /// Cards on offer the log does not name: not played.
    pub unknown_offers: usize,
}

/// One decision of the game and what became of it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Reviewed {
    pub decision: Decision,
    pub rebuilt: Result<Rebuilt, NotRebuilt>,
}

/// Every decision of a game, rebuilt or with the reason it is not.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct GameReview {
    pub decisions: Vec<Reviewed>,
    /// The time budget ran out, or the review was cancelled, before the end.
    pub out_of_time: bool,
    /// The review was cancelled.
    pub cancelled: bool,
    /// Boards played, each once however many decisions shared it.
    pub boards: usize,
    pub trials: u64,
}

/// Every decision of `report` (a saved game's report) played again, within
/// `budget`. `facts` gives the card data for a card id; `model` the
/// expected-place model, if any. Runs the simulator: call it off the
/// window's thread.
pub fn review(
    report: &Value,
    facts: &dyn Fn(&str) -> Option<Facts>,
    model: Option<&Model>,
    budget: &Budget,
) -> Result<GameReview, NoReview> {
    let started = Instant::now();
    let mut elapsed = || started.elapsed();
    run(
        report,
        facts,
        model,
        budget,
        &mut elapsed,
        None,
        &mut |_, _| {},
    )
}

/// [`review`] that stops, as when out of time, once `cancel` is set, and
/// calls `progress(done, total)` before each decision and once at the end.
pub fn review_with_progress(
    report: &Value,
    facts: &dyn Fn(&str) -> Option<Facts>,
    model: Option<&Model>,
    budget: &Budget,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(usize, usize),
) -> Result<GameReview, NoReview> {
    let started = Instant::now();
    let mut elapsed = || started.elapsed();
    run(
        report,
        facts,
        model,
        budget,
        &mut elapsed,
        Some(cancel),
        progress,
    )
}

/// [`review`] with the time since it began given by `elapsed` (for tests).
#[doc(hidden)]
pub fn review_with_clock(
    report: &Value,
    facts: &dyn Fn(&str) -> Option<Facts>,
    model: Option<&Model>,
    budget: &Budget,
    elapsed: &mut dyn FnMut() -> Duration,
) -> Result<GameReview, NoReview> {
    run(report, facts, model, budget, elapsed, None, &mut |_, _| {})
}

fn run(
    report: &Value,
    facts: &dyn Fn(&str) -> Option<Facts>,
    model: Option<&Model>,
    budget: &Budget,
    elapsed: &mut dyn FnMut() -> Duration,
    cancel: Option<&AtomicBool>,
    progress: &mut dyn FnMut(usize, usize),
) -> Result<GameReview, NoReview> {
    if report.get("game_type").and_then(Value::as_str) != Some("GT_BATTLEGROUNDS") {
        return Err(NoReview::NotSolo);
    }
    let list = decisions(report).map_err(NoReview::NoShop)?;
    let game = Game::new(report, facts, usable(model));
    let mut sims = Sims::new(*budget, elapsed, cancel);
    let played = |d: &Decision| {
        matches!(
            d.kind,
            DecisionKind::Buy | DecisionKind::Choose | DecisionKind::Sell
        )
    };
    let mut left = list.iter().filter(|d| played(d)).count();
    let total = list.len();
    let mut reviewed = Vec::with_capacity(total);
    for (i, decision) in list.iter().enumerate() {
        progress(i, total);
        if played(decision) {
            sims.start_decision(left);
            left -= 1;
        }
        reviewed.push(Reviewed {
            rebuilt: game.rebuild(&list, i, &mut sims),
            decision: decision.clone(),
        });
    }
    progress(total, total);
    Ok(GameReview {
        decisions: reviewed,
        out_of_time: sims.out_of_time,
        cancelled: sims.cancelled,
        boards: sims.boards(),
        trials: sims.trials,
    })
}

/// The model, or the marker every board gets instead of a number: none,
/// one players may not see yet, one of the other mode or format.
fn usable(model: Option<&Model>) -> Result<&Model, Place> {
    let m = model.ok_or(Place::NoModel)?;
    if !m.solo {
        return Err(Place::OtherMode);
    }
    let k = places(true);
    let shaped = m.encoder == expected_place::VERSION
        && m.mean.len() == expected_place::WIDTH
        && m.spread.len() == expected_place::WIDTH
        && m.weights.len() == k
        && m.weights.iter().all(|w| w.len() == INPUTS)
        && m.intercepts.len() == k;
    if !shaped {
        return Err(Place::OtherFormat);
    }
    if !m.ready() {
        return Err(Place::TooFewGames);
    }
    Ok(m)
}

/// The mean of a damage spread over every trial; None when a damage was
/// unknown.
fn mean_damage(o: &Odds, counts: &std::collections::BTreeMap<i32, u32>) -> Option<f64> {
    if o.damage_unknown > 0 || o.trials == 0 {
        return None;
    }
    let total: f64 = counts
        .iter()
        .map(|(d, n)| f64::from(*d) * f64::from(*n))
        .sum();
    Some(total / f64::from(o.trials))
}

/// A board's fight from its odds; `gifts` are the minions gifted with no
/// gift named, played as the plain card.
fn fight(o: &Odds, place: Place, gifts: &[String]) -> Fight {
    let mut approximate: Vec<String> = Vec::new();
    let ids = o.coverage.approximate.iter().map(|(id, _)| id).chain(gifts);
    for id in ids {
        if !approximate.contains(id) {
            approximate.push(id.clone());
        }
    }
    Fight {
        won: o.share(CombatResult::Won),
        tied: o.share(CombatResult::Tie),
        lost: o.share(CombatResult::Lost),
        trials: o.trials,
        stalled: o.stalled,
        minion_damage_dealt: mean_damage(o, &o.damage_dealt),
        minion_damage_taken: mean_damage(o, &o.damage_taken),
        place,
        approximate,
    }
}

/// The alternative against the actual board's trials when their
/// comparison stopped; `actual` is the fight of those trials.
fn delta(actual: &Fight, actual_odds: &Odds, alt: &Fight, alt_odds: &Odds) -> Delta {
    let minus = |a: Option<f64>, b: Option<f64>| Some(b? - a?);
    Delta {
        score: score(alt_odds) - score(actual_odds),
        standard_error: standard_error(actual_odds, alt_odds),
        clear: clear(actual_odds, alt_odds),
        actual_trials: actual_odds.trials,
        minion_damage_dealt: minus(actual.minion_damage_dealt, alt.minion_damage_dealt),
        minion_damage_taken: minus(actual.minion_damage_taken, alt.minion_damage_taken),
        place: minus(actual.place.number(), alt.place.number()),
    }
}

#[cfg(test)]
mod budget_tests;
#[cfg(test)]
mod sale_tests;
#[cfg(test)]
mod tests;
