//! The game review (T-658, D-176): the counterfactual engine plays each
//! choice of a saved game again in the next fight. Post-game only: it does
//! not start while a Battlegrounds game is being played, and a game that
//! starts while it runs cancels it before the engine's next batch (the game
//! tracking sets the flag the engine reads). One review at a time, on a
//! blocking thread, never the window's. The last answers are kept in memory
//! per game, so a window that reloads finds its review again.

use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde::Serialize;
use serde_json::{json, Value};
use tracker::counterfactual::{review_with_progress, Budget, GameReview, NoReview};
use tracker::decisions::{decisions, DecisionKind};
use tracker::odds::Facts;

/// What a review answers.
#[derive(Debug, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ReviewAnswer {
    /// A Battlegrounds game is being played: nothing ran.
    GameOn,
    NoReview {
        why: NoReview,
    },
    Done {
        review: GameReview,
        /// A game started while it ran, so it stopped early.
        stopped_for_game: bool,
        /// Card data attack and health of each card put in: the window's
        /// "after" board.
        stats: BTreeMap<String, [i32; 2]>,
    },
}

/// The event the main window gets before each choice is reviewed.
pub const PROGRESS: &str = "review-progress";
/// The event the main window gets when a review has its answer.
pub const DONE: &str = "review-done";
/// Answers kept, the last games reviewed.
const KEPT: usize = 8;

/// A game in the history.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GameRef {
    pub session: String,
    pub index: u64,
}

/// How far the review of one game is: choices the engine can simulate
/// (buys, picks, sales), not rolls or freezes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Progress {
    pub done: usize,
    pub total: usize,
}

struct Run {
    game: GameRef,
    progress: Progress,
}

#[derive(Default)]
struct Inner {
    running: Option<Run>,
    kept: VecDeque<(GameRef, Value)>,
}

/// The review running, if any, and the last answers.
#[derive(Default)]
pub struct Reviews {
    inner: Mutex<Inner>,
    /// Set when a game starts; read by the engine before each batch.
    cancel: AtomicBool,
}

/// What asking for a review gives.
pub enum Start {
    /// This review is the caller's to run.
    Started(Ticket),
    /// This game is already being reviewed.
    SameGame(Progress),
    /// Another game is being reviewed.
    Other,
}

/// The right to run one review; the running slot frees when it is dropped,
/// also on a panic.
pub struct Ticket {
    reviews: Arc<Reviews>,
    game: GameRef,
}

/// The window's "reviewing" state, with how far it is.
pub fn reviewing(p: Progress) -> Value {
    json!({"state": "reviewing", "done": p.done, "total": p.total})
}

impl Reviews {
    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn start(self: &Arc<Self>, game: &GameRef) -> Start {
        let mut inner = self.lock();
        match &inner.running {
            Some(run) if run.game == *game => Start::SameGame(run.progress),
            Some(_) => Start::Other,
            None => {
                inner.running = Some(Run {
                    game: game.clone(),
                    progress: Progress::default(),
                });
                // A game that started before this review is not its to stop.
                self.cancel.store(false, Ordering::Relaxed);
                Start::Started(Ticket {
                    reviews: Arc::clone(self),
                    game: game.clone(),
                })
            }
        }
    }

    /// The window's view of `game`: idle, reviewing or its last answer.
    pub fn status(&self, game: &GameRef) -> Value {
        let inner = self.lock();
        if let Some(run) = inner.running.as_ref().filter(|r| r.game == *game) {
            return reviewing(run.progress);
        }
        inner
            .kept
            .iter()
            .find(|(g, _)| g == game)
            .map_or_else(|| json!({"state": "idle"}), |(_, answer)| answer.clone())
    }

    /// A Battlegrounds game started: the review running stops.
    pub fn game_started(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

impl Ticket {
    /// The flag the engine reads before each batch.
    pub fn cancel(&self) -> &AtomicBool {
        &self.reviews.cancel
    }

    pub fn progress(&self, progress: Progress) {
        if let Some(run) = self.reviews.lock().running.as_mut() {
            run.progress = progress;
        }
    }

    /// Keeps the answer for the window and frees the running slot.
    pub fn finish(self, answer: Value) {
        let mut inner = self.reviews.lock();
        inner.kept.retain(|(g, _)| *g != self.game);
        inner.kept.push_back((self.game.clone(), answer));
        while inner.kept.len() > KEPT {
            inner.kept.pop_front();
        }
    }
}

impl Drop for Ticket {
    fn drop(&mut self) {
        let mut inner = self.reviews.lock();
        if inner.running.as_ref().is_some_and(|r| r.game == self.game) {
            inner.running = None;
        }
    }
}

/// The review of `report`, stopped when `cancel` is set or `game_on` turns
/// true (checked before each choice, should the game tracking miss it);
/// `progress` gets the choices that can be simulated, done and in all.
pub fn answer(
    report: &Value,
    facts: &dyn Fn(&str) -> Option<Facts>,
    game_on: &dyn Fn() -> bool,
    budget: &Budget,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(Progress),
) -> ReviewAnswer {
    if game_on() {
        return ReviewAnswer::GameOn;
    }
    let kinds: Vec<DecisionKind> = decisions(report)
        .map(|list| list.iter().map(|d| d.kind).collect())
        .unwrap_or_default();
    let simulated = simulated_prefix(&kinds);
    let total = simulated.last().copied().unwrap_or(0);
    // Called just before each choice is played: the last check of a game.
    let mut step = |done: usize, _all: usize| {
        let done = simulated.get(done).copied().unwrap_or(total);
        progress(Progress { done, total });
        if !cancel.load(Ordering::Relaxed) && game_on() {
            cancel.store(true, Ordering::Relaxed);
        }
    };
    match review_with_progress(report, facts, None, budget, cancel, &mut step) {
        Err(why) => ReviewAnswer::NoReview { why },
        Ok(review) => ReviewAnswer::Done {
            stats: stats_put_in(&review, facts),
            stopped_for_game: review.cancelled,
            review,
        },
    }
}

/// True for the choices the engine can play again: the others (rolls,
/// freezes, tier-ups, spells) are never simulated.
fn simulated(kind: DecisionKind) -> bool {
    matches!(
        kind,
        DecisionKind::Buy | DecisionKind::Choose | DecisionKind::Sell
    )
}

/// For each count of decisions done, how many of them can be simulated.
fn simulated_prefix(kinds: &[DecisionKind]) -> Vec<usize> {
    std::iter::once(0)
        .chain(kinds.iter().scan(0, |n, k| {
            *n += usize::from(simulated(*k));
            Some(*n)
        }))
        .collect()
}

/// Attack and health from the card data of every alternative card.
fn stats_put_in(
    review: &GameReview,
    facts: &dyn Fn(&str) -> Option<Facts>,
) -> BTreeMap<String, [i32; 2]> {
    review
        .decisions
        .iter()
        .filter_map(|d| d.rebuilt.as_ref().ok())
        .flat_map(|r| &r.alternatives)
        .filter_map(|a| {
            let f = facts(&a.card)?;
            Some((a.card.clone(), [f.base.attack, f.base.health]))
        })
        .collect()
}

#[cfg(test)]
#[path = "review_tests.rs"]
mod tests;
