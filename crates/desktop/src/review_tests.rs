use std::cell::Cell;

use super::*;

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

fn run(
    report: &Value,
    game_on: &dyn Fn() -> bool,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(Progress),
) -> ReviewAnswer {
    answer(
        report,
        &facts,
        game_on,
        &Budget::default(),
        cancel,
        progress,
    )
}

fn game(index: u64) -> GameRef {
    GameRef {
        session: "Hearthstone_2026_10_28_00_00_00".to_string(),
        index,
    }
}

#[test]
fn nothing_runs_while_a_game_is_being_played() {
    let a = run(
        &bought_weak("GT_BATTLEGROUNDS"),
        &|| true,
        &AtomicBool::new(false),
        &mut |_| {},
    );
    assert!(matches!(a, ReviewAnswer::GameOn));
}

#[test]
fn a_saved_game_is_reviewed_with_the_stats_of_each_card_put_in() {
    let mut seen = Vec::new();
    let a = run(
        &bought_weak("GT_BATTLEGROUNDS"),
        &|| false,
        &AtomicBool::new(false),
        &mut |p| seen.push(p),
    );
    assert_eq!(seen.last(), Some(&Progress { done: 1, total: 1 }));
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

/// Whatever the number of checks, a game that starts once the review runs
/// stops it, by the flag the game tracking sets or by the check of its own.
#[test]
fn a_game_starting_stops_the_review_and_says_so() {
    let cancel = AtomicBool::new(false);
    let a = run(
        &bought_weak("GT_BATTLEGROUNDS"),
        &|| false,
        &cancel,
        &mut |_| cancel.store(true, Ordering::Relaxed),
    );
    assert!(matches!(
        a,
        ReviewAnswer::Done {
            stopped_for_game: true,
            ..
        }
    ));

    let started = Cell::new(false);
    let a = run(
        &bought_weak("GT_BATTLEGROUNDS"),
        &|| started.get(),
        &AtomicBool::new(false),
        &mut |_| started.set(true),
    );
    let ReviewAnswer::Done {
        review,
        stopped_for_game,
        ..
    } = a
    else {
        panic!("not reviewed");
    };
    assert!(stopped_for_game && review.cancelled);
}

#[test]
fn a_game_that_is_not_solo_gets_no_review() {
    let a = run(
        &bought_weak("GT_BATTLEGROUNDS_DUO"),
        &|| false,
        &AtomicBool::new(false),
        &mut |_| {},
    );
    let json = serde_json::to_value(&a).unwrap();
    assert_eq!(json, json!({"state": "no_review", "why": "not_solo"}));
}

#[test]
fn progress_counts_only_the_choices_that_can_be_simulated() {
    use DecisionKind::*;
    // Done 0..=5 of: roll, buy, freeze, sell, choose.
    assert_eq!(
        simulated_prefix(&[Roll, Buy, Freeze, Sell, Choose]),
        [0, 0, 1, 1, 2, 3]
    );
    assert_eq!(simulated_prefix(&[]), [0]);
}

#[test]
fn one_review_at_a_time_and_the_same_game_is_found_again() {
    let reviews = Arc::new(Reviews::default());
    assert_eq!(reviews.status(&game(1)), json!({"state": "idle"}));
    let Start::Started(ticket) = reviews.start(&game(1)) else {
        panic!("none running");
    };
    ticket.progress(Progress { done: 2, total: 5 });
    assert!(matches!(reviews.start(&game(2)), Start::Other));
    assert!(matches!(
        reviews.start(&game(1)),
        Start::SameGame(Progress { done: 2, total: 5 })
    ));
    assert_eq!(
        reviews.status(&game(1)),
        json!({"state": "reviewing", "done": 2, "total": 5})
    );
    ticket.finish(json!({"state": "done", "n": 1}));
    assert_eq!(
        reviews.status(&game(1)),
        json!({"state": "done", "n": 1}),
        "the answer is kept, for a window that reloads"
    );
    assert!(matches!(reviews.start(&game(2)), Start::Started(_)));
}

#[test]
fn a_review_that_ends_without_an_answer_frees_its_place() {
    let reviews = Arc::new(Reviews::default());
    let Start::Started(ticket) = reviews.start(&game(1)) else {
        panic!("none running");
    };
    drop(ticket);
    assert_eq!(reviews.status(&game(1)), json!({"state": "idle"}));
    assert!(matches!(reviews.start(&game(1)), Start::Started(_)));
}

#[test]
fn a_game_starting_cancels_the_review_running_and_the_next_starts_clear() {
    let reviews = Arc::new(Reviews::default());
    let Start::Started(ticket) = reviews.start(&game(1)) else {
        panic!("none running");
    };
    reviews.game_started();
    assert!(ticket.cancel().load(Ordering::Relaxed));
    ticket.finish(json!({"state": "done"}));
    let Start::Started(next) = reviews.start(&game(2)) else {
        panic!("none running");
    };
    assert!(!next.cancel().load(Ordering::Relaxed));
}

#[test]
fn only_the_last_answers_are_kept() {
    let reviews = Arc::new(Reviews::default());
    for i in 0..(KEPT as u64 + 2) {
        let Start::Started(t) = reviews.start(&game(i)) else {
            panic!("none running");
        };
        t.finish(json!({"state": "done", "i": i}));
    }
    assert_eq!(reviews.status(&game(0)), json!({"state": "idle"}));
    assert_eq!(reviews.status(&game(KEPT as u64 + 1))["i"], KEPT as u64 + 1);
    // A game reviewed again keeps one answer, the new one.
    let Start::Started(t) = reviews.start(&game(5)) else {
        panic!("none running");
    };
    t.finish(json!({"state": "done", "again": true}));
    assert_eq!(reviews.status(&game(5))["again"], true);
}
