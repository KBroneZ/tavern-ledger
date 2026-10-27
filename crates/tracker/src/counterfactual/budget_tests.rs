//! The counterfactual engine's time budget, cache and early stop.

use super::sim::Sims;
use super::tests::*;
use super::*;
use combat_sim::{Team, Warband};
use std::sync::atomic::Ordering;

/// A long synthetic game: every turn a buy with five other cards on offer,
/// on full boards. The card bought alternates, so each is
/// new on its turn's board.
fn long_game(turns: i64) -> Value {
    let offers = [WEAK, STRONG, OTHER, FILLER, FOE, STRONG];
    let bought = |t: i64| if t % 2 == 1 { WEAK } else { OTHER };
    let shop: Vec<Value> = (1..=turns)
        .map(|t| shop_turn(t, &[bought(t)], &[], &offers))
        .collect();
    let actions: Vec<Value> = (1..=turns).map(|t| act(t, 0, "buy")).collect();
    let rounds: Vec<Value> = (1..=turns)
        .map(|t| {
            let mut own: Vec<Value> = (0..6).map(|i| minion(FILLER, 2 + i, 3)).collect();
            own.push(minion(bought(t), 2, 2));
            // Another opponent each turn: no board is played twice.
            let theirs: Vec<Value> = (0..7).map(|i| minion(FOE, 3 + i % 3, 2 + t)).collect();
            round(t, own, theirs)
        })
        .collect();
    report(shop, actions, rounds)
}

#[test]
fn the_time_budget_is_respected_and_what_it_did_not_reach_says_so() {
    let g = long_game(12);
    let budget = Budget {
        time: Duration::from_millis(10),
        max_trials: 2_000,
        batch: 100,
    };
    // A clock that moves one millisecond each time it is read.
    let mut now = 0u64;
    let mut clock = || {
        now += 1;
        Duration::from_millis(now)
    };

    let r = review_with_clock(&g, &facts, None, &budget, &mut clock).unwrap();

    assert!(r.out_of_time);
    // Every decision is there, rebuilt or with its reason.
    assert_eq!(r.decisions.len(), 12);
    // The clock is read before each batch, and never after the time is up:
    // at most 9 batches ran.
    assert!(r.trials <= 9 * u64::from(budget.batch), "{}", r.trials);
    let last = r.decisions.last().unwrap();
    assert_eq!(last.rebuilt, Err(NotRebuilt::Actual(Gap::OutOfTime)));
    let out_of_time = r
        .decisions
        .iter()
        .filter(|d| d.rebuilt == Err(NotRebuilt::Actual(Gap::OutOfTime)))
        .count();
    assert!(out_of_time > 0);
}

#[test]
fn a_real_clock_budget_stops_the_review_in_time() {
    let g = long_game(16);
    let budget = Budget {
        time: Duration::from_millis(50),
        max_trials: 1_000_000,
        batch: 200,
    };
    let started = Instant::now();

    let r = review(&g, &facts, None, &budget).unwrap();

    assert!(r.out_of_time);
    assert_eq!(r.decisions.len(), 16);
    // One batch may run past the budget; a batch of 200 trials is far
    // below this, even on a slow machine.
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "{:?}",
        started.elapsed()
    );
}

/// A game of close, random fights: every turn a buy with four other cards
/// on offer, one of the opponent's minions tougher each turn so no board is
/// played twice. A 1/1 in place of the 1/1 bought is the same fight again:
/// never clear.
fn close_game(turns: i64) -> Value {
    let offers = [WEAK, STRONG, OTHER, FILLER, FOE];
    let bought = |t: i64| if t % 2 == 1 { WEAK } else { OTHER };
    let shop: Vec<Value> = (1..=turns)
        .map(|t| shop_turn(t, &[bought(t)], &[], &offers))
        .collect();
    let actions: Vec<Value> = (1..=turns).map(|t| act(t, 0, "buy")).collect();
    let rounds: Vec<Value> = (1..=turns)
        .map(|t| {
            let own = vec![
                minion(FILLER, 4, 3),
                minion(FILLER, 2, 5),
                minion(bought(t), 1, 1),
            ];
            let theirs = vec![minion(FOE, 3, 3), minion(FOE, 1, 5 + t), minion(FOE, 5, 2)];
            round(t, own, theirs)
        })
        .collect();
    report(shop, actions, rounds)
}

#[test]
fn each_decision_gets_its_share_of_the_time() {
    let g = close_game(12);
    let budget = Budget {
        time: Duration::from_millis(400),
        max_trials: 1_000_000,
        batch: 100,
    };
    let mut now = 0u64;
    let mut clock = || {
        now += 1;
        Duration::from_millis(now)
    };

    let r = review_with_clock(&g, &facts, None, &budget, &mut clock).unwrap();

    // The first decisions' unclear comparisons stop at their share of the
    // time, so the last decision is still played in full.
    let first = rebuilt(&r, 0);
    assert!(first
        .alternatives
        .iter()
        .any(|a| a.stop == Some(Stop::OutOfTime)));
    let last = rebuilt(&r, 11);
    assert!(
        last.alternatives.iter().all(|a| a.fight.is_ok()),
        "{:?}",
        last.alternatives
    );
}

#[test]
fn a_delta_is_clear_exactly_when_its_comparison_stopped_clear() {
    let budget = Budget {
        max_trials: 2_000,
        batch: 250,
        ..Budget::default()
    };

    let r = review(&close_game(4), &facts, None, &budget).unwrap();

    let mut seen = 0;
    for d in &r.decisions {
        let b = d.rebuilt.as_ref().unwrap();
        for a in &b.alternatives {
            let Some(delta) = &a.delta else { continue };
            seen += 1;
            assert_eq!(delta.clear, a.stop == Some(Stop::Clear), "{a:?}");
            assert!(delta.actual_trials <= b.actual.trials);
            let f = a.fight.as_ref().unwrap();
            let actual_score = f.score() - delta.score;
            assert!((0.0..=1.0).contains(&actual_score));
        }
    }
    assert!(seen > 8);
}

#[test]
fn boards_that_differ_in_one_field_are_played_apart() {
    let board = |health: i32| {
        let m = combat_sim::Minion::new(FOE, 3, health);
        (
            Team::solo(Warband::new(vec![m.clone()])),
            Team::solo(Warband::new(vec![m])),
        )
    };
    let mut clock = || Duration::ZERO;
    let mut sims = Sims::new(Budget::default(), &mut clock, None);

    sims.play(&board(3)).unwrap();
    sims.play(&board(4)).unwrap();
    sims.play(&board(3)).unwrap();

    assert_eq!(sims.boards(), 2);
    assert_eq!(sims.trials, 2 * u64::from(Budget::default().batch));
}

#[test]
fn a_cancelled_review_stops_and_reports_its_progress() {
    let g = long_game(4);
    let cancel = AtomicBool::new(true);
    let mut seen = Vec::new();

    let r = review_with_progress(
        &g,
        &facts,
        None,
        &Budget::default(),
        &cancel,
        &mut |done, total| seen.push((done, total)),
    )
    .unwrap();

    assert!(r.cancelled && r.out_of_time);
    assert_eq!(r.trials, 0);
    assert!(r
        .decisions
        .iter()
        .all(|d| d.rebuilt == Err(NotRebuilt::Actual(Gap::OutOfTime))));
    assert_eq!(seen, [(0, 4), (1, 4), (2, 4), (3, 4), (4, 4)]);

    cancel.store(false, Ordering::Relaxed);
    let r = review_with_progress(
        &g,
        &facts,
        None,
        &Budget::default(),
        &cancel,
        &mut |_, _| {},
    )
    .unwrap();
    assert!(!r.cancelled);
}
