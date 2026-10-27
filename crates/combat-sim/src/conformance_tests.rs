//! The conformance check on hand-made corpus combats: what it records for a
//! replay that follows the game and one that leaves it, and how a run is
//! compared with the baseline.

use serde_json::json;

use super::*;
use crate::corpus::{export, read_line};
use crate::saved::CardFacts;

/// A card with no combat text (`cards::NO_COMBAT_EFFECT`).
const PLAIN: &str = "BG28_603t";

fn db() -> CardDb {
    CardDb::from([(
        PLAIN.to_string(),
        CardFacts {
            tier: Some(1),
            tribes: crate::model::Tribes::NONE,
            base: None,
        },
    )])
}

fn bm(card: &str, atk: i64, health: i64, position: i64) -> Value {
    json!({"card_id": card, "golden": false, "atk": atk, "health": health,
           "position": position, "keywords": []})
}

/// The player's 3/3 against a 1/1: the 3/3 attacks, kills it and wins.
/// `opponent_first`: the saved replay has the 1/1 attack first instead,
/// which the engine never does (a side with as many minions flips a coin,
/// so give the player two minions to make it attack first).
fn corpus_combat(opponent_first: bool) -> CorpusCombat {
    let on = |side: &str, mut m: Value| {
        m["side"] = json!(side);
        m
    };
    let events = if opponent_first {
        json!([{"kind": "attack", "attacker": 2, "defender": 0}])
    } else {
        json!([
            {"kind": "attack", "attacker": 0, "defender": 2},
            {"kind": "damage", "minion": 2, "amount": 3, "health": -2},
            {"kind": "damage", "minion": 0, "amount": 1, "health": 2},
            {"kind": "death", "minion": 2},
        ])
    };
    let round = json!({
        "number": 1,
        "result": "won",
        "entries": [
            {"side": "own", "player_id": 1, "board": [bm(PLAIN, 3, 3, 1), bm(PLAIN, 1, 1, 2)]},
            {"side": "opponent", "player_id": 2, "board": [bm(PLAIN, 1, 1, 1)]},
        ],
        "replay": {
            "complete": true,
            "start": 3,
            "minions": [
                on("own", bm(PLAIN, 3, 3, 1)),
                on("own", bm(PLAIN, 1, 1, 2)),
                on("opponent", bm(PLAIN, 1, 1, 1)),
            ],
            "events": events,
        },
    });
    let history = json!({"session": "S", "index": 1, "report": {
        "game_type": "GT_BATTLEGROUNDS", "build": 254_477, "rounds": [round]}})
    .to_string();
    read_line(&export(&history).unwrap().lines[0]).unwrap()
}

fn options() -> Options {
    Options {
        trials: 200,
        ..Options::default()
    }
}

fn outcome(c: &CorpusCombat) -> Outcome {
    check(c, &|id| db().get(id).copied(), &options()).unwrap()
}

#[test]
fn a_replay_that_follows_the_game_has_no_divergence() {
    let o = outcome(&corpus_combat(false));
    assert_eq!(o.said, "odds");
    assert_eq!(o.likely.as_deref(), Some("won"));
    assert_eq!(o.result, "won");
    assert!(!o.close);
    let r = o.replay.expect("the replay is readable");
    assert_eq!(r.diverge, None);
    assert_eq!(r.attacks, r.matched);
}

#[test]
fn a_replay_that_leaves_the_game_says_where_even_when_the_odds_were_right() {
    let o = outcome(&corpus_combat(true));
    // The likely result happened, so the accuracy page counts it as right;
    // the replay still shows the engine is not the game here.
    assert_eq!(o.likely.as_deref(), Some("won"));
    let d = o
        .replay
        .unwrap()
        .diverge
        .expect("parts at the first attack");
    assert_eq!(d.attack, 1);
    assert_eq!(d.rule, "replay_first_attacker");
}

#[test]
fn a_build_without_card_data_is_an_error() {
    let corpus = [corpus_combat(false)];
    let err = run(&corpus, &BTreeMap::new(), &options()).unwrap_err();
    assert!(err.contains("254477"), "{err}");
    let cards = BTreeMap::from([(254_477, db())]);
    assert_eq!(run(&corpus, &cards, &options()).unwrap().len(), 1);
}

#[test]
fn the_baseline_reads_back_as_written() {
    let cards = BTreeMap::from([(254_477, db())]);
    let corpus = [corpus_combat(false), corpus_combat(true)];
    let outcomes = run(&corpus, &cards, &options()).unwrap();
    let text = baseline_json(&outcomes, &options());
    let (o, back) = read_baseline(&text).unwrap();
    assert_eq!(back, outcomes);
    assert_eq!((o.trials, o.seed), (200, options().seed));
    assert!(compare(&back, &outcomes).changes.is_empty());
    assert!(read_baseline("{\"format\": 9}").is_err());
}

fn base() -> Outcome {
    Outcome {
        build: 1,
        duos: false,
        said: "odds".into(),
        likely: Some("won".into()),
        result: "won".into(),
        close: false,
        replay: Some(ReplayCheck {
            attacks: 6,
            matched: 2,
            diverge: Some(Diverge {
                attack: 3,
                rule: "replay_stats".into(),
                card: Some("BG28_603t".into()),
            }),
        }),
    }
}

fn kind_of(old: Outcome, new: Outcome) -> Option<Kind> {
    let b = BTreeMap::from([("a".to_string(), old)]);
    let n = BTreeMap::from([("a".to_string(), new)]);
    let diff = compare(&b, &n);
    diff.changes.first().map(|c| c.kind)
}

fn at(attack: Option<usize>, rule: &str) -> Outcome {
    let mut o = base();
    o.replay = Some(ReplayCheck {
        attacks: 6,
        matched: attack.map_or(6, |a| a - 1),
        diverge: attack.map(|attack| Diverge {
            attack,
            rule: rule.into(),
            card: None,
        }),
    });
    o
}

#[test]
fn a_replay_that_parts_earlier_is_a_regression_and_later_an_improvement() {
    assert_eq!(
        kind_of(at(Some(3), "replay_stats"), at(Some(2), "replay_stats")),
        Some(Kind::Regression)
    );
    assert_eq!(
        kind_of(at(Some(3), "replay_stats"), at(Some(5), "replay_stats")),
        Some(Kind::Improvement)
    );
    assert_eq!(
        kind_of(at(Some(3), "replay_stats"), at(None, "")),
        Some(Kind::Improvement)
    );
    assert_eq!(
        kind_of(at(None, ""), at(Some(6), "replay_end")),
        Some(Kind::Regression)
    );
    assert_eq!(
        kind_of(at(Some(3), "replay_stats"), at(Some(3), "replay_minions")),
        Some(Kind::Changed)
    );
    assert_eq!(kind_of(base(), base()), None);
}

#[test]
fn odds_lost_or_a_likely_result_lost_is_a_regression() {
    let mut refused = base();
    refused.said = "not_modelled".into();
    refused.likely = None;
    assert_eq!(kind_of(base(), refused.clone()), Some(Kind::Regression));
    assert_eq!(kind_of(refused, base()), Some(Kind::Improvement));
    let mut wrong = base();
    wrong.likely = Some("lost".into());
    assert_eq!(kind_of(base(), wrong.clone()), Some(Kind::Regression));
    assert_eq!(kind_of(wrong, base()), Some(Kind::Improvement));
}

#[test]
fn a_close_combat_that_flips_is_listed_but_does_not_fail() {
    let mut old = base();
    old.close = true;
    let mut new = old.clone();
    new.likely = Some("lost".into());
    let b = BTreeMap::from([("a".to_string(), old)]);
    let n = BTreeMap::from([("a".to_string(), new)]);
    let diff = compare(&b, &n);
    assert_eq!(diff.changes[0].kind, Kind::CloseFlip);
    assert!(!diff.fails());
    assert!(render_diff(&diff).contains("not a failure"));
}

#[test]
fn every_other_difference_fails_and_names_the_combat() {
    let b = BTreeMap::from([
        ("gone".to_string(), base()),
        ("a".to_string(), at(Some(3), "replay_stats")),
    ]);
    let n = BTreeMap::from([
        ("a".to_string(), at(Some(5), "replay_stats")),
        ("new".to_string(), base()),
    ]);
    let diff = compare(&b, &n);
    assert!(diff.fails());
    assert_eq!(diff.added, ["new"]);
    assert_eq!(diff.removed, ["gone"]);
    let text = render_diff(&diff);
    assert!(
        text.contains(
            "improvement: a leaves at attack 3 (replay_stats) -> leaves at attack 5 (replay_stats)"
        ),
        "{text}"
    );
    assert!(text.contains("new in the corpus: new"), "{text}");
    assert!(text.contains("gone from the corpus: gone"), "{text}");
}

#[test]
fn the_summary_counts_per_mode() {
    let outcomes = BTreeMap::from([("a".to_string(), base()), ("b".to_string(), at(None, ""))]);
    let text = render_summary(&outcomes);
    assert!(text.contains("Solo: 2 combats"), "{text}");
    assert!(
        text.contains("likely result right: 2 of 2 (100.0%)"),
        "{text}"
    );
    assert!(
        text.contains("follow the game to the end: 1 of 2 (50.0%)"),
        "{text}"
    );
    assert!(text.contains("first difference replay_stats: 1"), "{text}");
    assert!(!text.contains("Duos"), "{text}");
}

#[test]
fn two_runs_give_the_same_outcomes() {
    let cards = BTreeMap::from([(254_477, db())]);
    let corpus = [corpus_combat(false), corpus_combat(true)];
    let a = run(&corpus, &cards, &options()).unwrap();
    let b = run(&corpus, &cards, &options()).unwrap();
    assert_eq!(a, b);
    assert_eq!(baseline_json(&a, &options()), baseline_json(&b, &options()));
}

#[test]
fn a_replay_lost_or_found_is_classed_by_what_it_did() {
    let mut gone = base();
    gone.replay = None;
    // A replay that followed the game and can no longer be checked.
    assert_eq!(kind_of(at(None, ""), gone.clone()), Some(Kind::Regression));
    assert_eq!(kind_of(gone.clone(), at(None, "")), Some(Kind::Improvement));
    // One that already left the game: nothing better or worse to say.
    assert_eq!(
        kind_of(at(Some(3), "replay_stats"), gone),
        Some(Kind::Changed)
    );
}

#[test]
fn a_clear_combat_that_becomes_close_and_flips_fails() {
    let mut new = base();
    new.close = true;
    new.likely = Some("lost".into());
    assert_eq!(kind_of(base(), new), Some(Kind::Regression));
}

#[test]
fn every_part_that_moved_is_named() {
    let mut new = at(Some(5), "replay_stats");
    new.said = "not_modelled".into();
    new.likely = None;
    let b = BTreeMap::from([("a".to_string(), at(Some(3), "replay_stats"))]);
    let n = BTreeMap::from([("a".to_string(), new)]);
    let c = &compare(&b, &n).changes[0];
    assert_eq!(
        c.kind,
        Kind::Regression,
        "odds lost outweighs a later difference"
    );
    assert!(c.what.contains("leaves at attack 5") && c.what.contains("said odds -> not_modelled"));
}

#[test]
fn a_malformed_baseline_is_refused() {
    let cards = BTreeMap::from([(254_477, db())]);
    let outcomes = run(&[corpus_combat(false)], &cards, &options()).unwrap();
    let good: Value = serde_json::from_str(&baseline_json(&outcomes, &options())).unwrap();
    let id = outcomes.keys().next().unwrap().clone();
    let mut cases: Vec<Value> = Vec::new();
    for (path, value) in [
        ("mode", json!("trio")),
        ("said", json!("maybe")),
        ("likely", json!(3)),
        ("close", json!("no")),
        ("build", json!("x")),
    ] {
        let mut v = good.clone();
        v["combats"][&id][path] = value;
        cases.push(v);
    }
    let mut v = good.clone();
    v["trials"] = json!(0);
    cases.push(v);
    let mut v = good.clone();
    v["combats"][&id]["replay"]["diverge"] = json!({"rule": "replay_stats"});
    cases.push(v);
    for bad in cases {
        assert!(read_baseline(&bad.to_string()).is_err(), "{bad}");
    }
    assert!(read_baseline(&good.to_string()).is_ok());
}
