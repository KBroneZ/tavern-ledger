//! The game review (T-658, D-176): the counterfactual engine plays each
//! choice of a saved game again in the next fight. Post-game only: it does
//! not start while a Battlegrounds game is being played, and a game that
//! starts while it runs cancels the choices not reviewed yet. One review at
//! a time; it runs on a blocking thread, never the window's.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;
use serde_json::Value;
use tracker::counterfactual::{review_with_progress, Budget, GameReview, NoReview};
use tracker::odds::Facts;

/// What the window gets back.
#[derive(Debug, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ReviewAnswer {
    /// A Battlegrounds game is being played: nothing ran.
    GameOn,
    /// Another review is running.
    Busy,
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

/// How far the review of one game is.
#[derive(Clone, Serialize)]
pub struct Progress<'a> {
    pub session: &'a str,
    pub index: u64,
    pub done: usize,
    pub total: usize,
}

/// True while a review runs: one at a time, as each takes up to the budget.
static RUNNING: AtomicBool = AtomicBool::new(false);

/// Held while a review runs; lets the next one start when dropped, also on
/// a panic.
pub struct Running(());

impl Running {
    /// None when another review is running.
    pub fn start() -> Option<Running> {
        RUNNING
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()
            .map(|_| Running(()))
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        RUNNING.store(false, Ordering::Release);
    }
}

/// The review of `report`, cancelled when `game_on` turns true; `progress`
/// gets (choices done, choices in all) before each choice and at the end.
pub fn answer(
    report: &Value,
    facts: &dyn Fn(&str) -> Option<Facts>,
    game_on: &dyn Fn() -> bool,
    budget: &Budget,
    progress: &mut dyn FnMut(usize, usize),
) -> ReviewAnswer {
    if game_on() {
        return ReviewAnswer::GameOn;
    }
    let cancel = AtomicBool::new(false);
    // Checked before each choice: a game that starts cancels the rest, which
    // the engine then marks as not reached.
    let mut step = |done: usize, total: usize| {
        if !cancel.load(Ordering::Relaxed) && game_on() {
            cancel.store(true, Ordering::Relaxed);
        }
        progress(done, total);
    };
    match review_with_progress(report, facts, None, budget, &cancel, &mut step) {
        Err(why) => ReviewAnswer::NoReview { why },
        Ok(review) => ReviewAnswer::Done {
            stats: stats_put_in(&review, facts),
            stopped_for_game: review.cancelled,
            review,
        },
    }
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
mod tests {
    use super::*;
    use serde_json::json;
    use std::cell::Cell;

    // Cards with no combat text in the engine.
    const WEAK: &str = "BG28_603t";
    const STRONG: &str = "BG26_361";
    const FOE: &str = "BGS_119";

    fn facts(id: &str) -> Option<Facts> {
        let (a, h) = match id {
            WEAK => (1, 1),
            STRONG => (20, 20),
            FOE => (3, 3),
            _ => return None,
        };
        Some(Facts::from_names(a, h, Some(1), &[], &[]))
    }

    fn minion(card: &str, atk: i64, health: i64) -> Value {
        json!({"card_id": card, "atk": atk, "health": health, "golden": false, "keywords": []})
    }

    /// Turn 1: the shop shows WEAK and STRONG; the player buys WEAK.
    fn bought_weak(game_type: &str) -> Value {
        json!({
            "status": "ok", "game_type": game_type, "local_player_id": 1,
            "final_place": 4, "start_health": {"1": 30, "2": 30},
            "shop": {"turns": [{
                "turn": 1, "start_ms": 1000, "end_ms": 1999, "tier": 1, "gold": 3,
                "gold_spent": 3, "rolls": 0, "free_rolls": 0, "buys": [WEAK], "spell_buys": 0,
                "sells": [], "freezes": 0, "actions": 0,
                "offers": [{"card_id": WEAK, "roll": 0, "frozen": false},
                           {"card_id": STRONG, "roll": 0, "frozen": false}],
            }], "tier_ups": [], "actions": [{"ms": 1100, "turn": 1, "kind": "buy"}],
                     "start_ms": 0, "end_ms": 99_000, "ended": true},
            "discovers": [],
            "rounds": [{"number": 1, "result": "lost", "entries": [
                {"side": "own", "player_id": 1, "board": [minion(WEAK, 1, 1)]},
                {"side": "opponent", "player_id": 2, "board": [minion(FOE, 3, 3)]},
            ], "health_after": {"1": 30, "2": 25}}],
        })
    }

    #[test]
    fn nothing_runs_while_a_game_is_being_played() {
        let a = answer(
            &bought_weak("GT_BATTLEGROUNDS"),
            &facts,
            &|| true,
            &Budget::default(),
            &mut |_, _| {},
        );
        assert!(matches!(a, ReviewAnswer::GameOn));
    }

    #[test]
    fn a_saved_game_is_reviewed_with_the_stats_of_each_card_put_in() {
        let mut seen = Vec::new();
        let a = answer(
            &bought_weak("GT_BATTLEGROUNDS"),
            &facts,
            &|| false,
            &Budget::default(),
            &mut |done, total| seen.push((done, total)),
        );
        assert_eq!(seen.last(), Some(&(1, 1)), "progress reaches the end");
        let ReviewAnswer::Done {
            review,
            stopped_for_game,
            stats,
        } = a
        else {
            panic!("not reviewed");
        };
        assert!(!stopped_for_game);
        assert_eq!(review.decisions.len(), 1);
        assert_eq!(stats.get(STRONG), Some(&[20, 20]));
        let json = serde_json::to_value(ReviewAnswer::Done {
            review,
            stopped_for_game,
            stats,
        })
        .unwrap();
        assert_eq!(json["state"], "done");
        assert!(
            json["review"]["decisions"][0]["rebuilt"]["Ok"].is_object(),
            "{json}"
        );
    }

    #[test]
    fn a_game_starting_stops_the_review_and_says_so() {
        let calls = Cell::new(0);
        // Free at the check before it starts, a game on from then.
        let game_on = || {
            calls.set(calls.get() + 1);
            calls.get() > 1
        };
        let a = answer(
            &bought_weak("GT_BATTLEGROUNDS"),
            &facts,
            &game_on,
            &Budget::default(),
            &mut |_, _| {},
        );
        let ReviewAnswer::Done {
            review,
            stopped_for_game,
            ..
        } = a
        else {
            panic!("not reviewed");
        };
        assert!(stopped_for_game);
        assert!(review.cancelled && review.out_of_time);
    }

    #[test]
    fn one_review_runs_at_a_time() {
        let first = Running::start().expect("none running");
        assert!(Running::start().is_none(), "a second waits");
        drop(first);
        assert!(Running::start().is_some(), "free again once the first ends");
    }

    #[test]
    fn a_game_that_is_not_solo_gets_no_review() {
        let a = answer(
            &bought_weak("GT_BATTLEGROUNDS_DUO"),
            &facts,
            &|| false,
            &Budget::default(),
            &mut |_, _| {},
        );
        assert!(matches!(
            a,
            ReviewAnswer::NoReview {
                why: NoReview::NotSolo
            }
        ));
        let json = serde_json::to_value(&a).unwrap();
        assert_eq!(json, json!({"state": "no_review", "why": "not_solo"}));
    }
}
