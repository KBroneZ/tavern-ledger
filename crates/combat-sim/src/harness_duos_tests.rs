//! The Duos harness on a synthetic saved game: seats 1 and 2 are the
//! player's team (1 is the player), 3 and 4 the opposing team.

use serde_json::{json, Value};

use super::*;
use crate::harness::load_cards;

const CARDS: &str = include_str!("../tests/data/cards.json");

/// A board of plain minions (`BGS_119` has no combat effect).
fn board(stats: &[(i64, i64)]) -> Value {
    let minions: Vec<Value> = stats
        .iter()
        .enumerate()
        .map(|(i, (atk, health))| {
            json!({"card_id": "BGS_119", "atk": atk, "health": health, "position": i + 1})
        })
        .collect();
    Value::Array(minions)
}

fn entry(side: &str, seat: i64, b: Option<Value>) -> Value {
    json!({"side": side, "player_id": seat, "board": b})
}

fn game() -> Value {
    let lobby: Vec<Value> = (1..=4)
        .map(|s| json!({"player_id": s, "duo_team": if s <= 2 { 1 } else { 2 }}))
        .collect();
    let rounds = vec![
        // Round 1: the player's board (seat 1) settles it alone against
        // seat 3; seat 4 comes in but no attack follows (no board).
        json!({"number": 1, "result": "won", "health_after": {"1": 30, "4": 20},
               "entries": [entry("own", 1, Some(board(&[(9, 9)]))),
                           entry("opponent", 3, Some(board(&[(1, 1)]))),
                           entry("opponent", 4, None)]}),
        // Round 2: the teammate (seat 2) fights first, then the player;
        // seat 4 first, then seat 3. The player's switch back has no board.
        json!({"number": 2, "result": "lost", "health_after": {"3": 0},
               "entries": [entry("own", 2, Some(board(&[(1, 1)]))),
                           entry("opponent", 4, Some(board(&[(5, 5)]))),
                           entry("own", 1, Some(board(&[(2, 2)]))),
                           entry("opponent", 3, Some(board(&[(3, 3)]))),
                           entry("own", 2, None)]}),
        // Round 3: seat 3 is out; the player first, then the teammate.
        json!({"number": 3, "result": "won",
               "entries": [entry("own", 1, Some(board(&[(9, 9), (9, 9)]))),
                           entry("opponent", 4, Some(board(&[(1, 1)])))]}),
        // A round whose card id is unreadable is left out.
        json!({"number": 4, "result": "won",
               "entries": [entry("own", 1, Some(json!([{"atk": 1}]))),
                           entry("opponent", 4, Some(board(&[(1, 1)])))]}),
    ];
    json!({"game_type": "GT_BATTLEGROUNDS_DUO", "local_player_id": 1,
           "teammate_player_id": 2, "lobby": lobby, "rounds": rounds})
}

fn combats() -> Vec<DuosCombat> {
    let db = load_cards(CARDS).unwrap();
    duos_from_saved(&game(), 7, &db)
}

#[test]
fn each_team_keeps_its_boards_in_the_order_they_came_in() {
    let all = combats();
    assert_eq!(all.len(), 3);
    let r2 = &all[1];
    assert_eq!((r2.game, r2.round), (7, 2));
    let seats = |legs: &[Leg]| legs.iter().map(|l| l.seat).collect::<Vec<_>>();
    assert_eq!(
        seats(&r2.own),
        [Some(2), Some(1)],
        "the switch back is not a leg"
    );
    assert_eq!(seats(&r2.opponent), [Some(4), Some(3)]);
    assert_eq!(r2.opponent_seats, [3, 4]);
    assert_eq!(r2.result, Some(CombatResult::Lost));
    // The history has no shop board: the player's board in the combat.
    assert_eq!(r2.own_shop.as_ref().unwrap().minions[0].attack, 2);
    // A board with no attack after it came in is not a leg.
    assert_eq!(seats(&all[0].opponent), [Some(3)]);
    // Health 0 after round 2: out of the game in round 3.
    assert_eq!(all[2].out, [3]);
    let team = team_of(&r2.own);
    assert_eq!(team.first.minions[0].attack, 1);
    assert_eq!(team.second.unwrap().minions[0].attack, 2);
}

#[test]
fn the_report_scores_both_paths_and_counts_what_is_missing() {
    let all = combats();
    let opts = Options {
        trials: 300,
        ..Options::default()
    };
    let text = render_duos(&all, &opts, Source::History);
    assert!(text.contains("every card id readable: 3 (3 with"), "{text}");
    assert!(text.contains("both teams' two boards 1"), "{text}");
    // Rounds 1 and 2: no board of the teammate seen in an earlier combat
    // (round 2's own is the one it fights with, not one seen before).
    assert!(
        text.contains("Teammate's board never seen before: 2"),
        "{text}"
    );
    assert!(text.contains("Scored: 1."), "{text}");
    // Round 3: the player's 9/9s and the teammate's 1/1 from round 2
    // against seat 4's 5/5 from round 2, seat 3 out: a certain win, both
    // boards one round old.
    assert!(
        text.contains("| 1 | 100% | 0.000 | 1 of 1 | 1.0 |"),
        "{text}"
    );
    assert!(!text.contains("BGS_119"), "no card ids needed here");
}

#[test]
fn a_board_that_never_came_in_leaves_the_team_with_one() {
    let all = combats();
    let r1 = &all[0];
    let t = team_of(&r1.own);
    assert_eq!(t.second, None);
}
