//! The encoder on hand-made saved games.

use serde_json::{json, Value};

use super::*;

fn minion(atk: i64, health: i64, keywords: &[&str]) -> Value {
    json!({"card_id": "BG28_603t", "atk": atk, "health": health, "golden": false,
           "position": 1, "keywords": keywords})
}

/// A Solo game of two combats; the player is seat 3, finishing 2nd.
fn game() -> Value {
    json!({
        "game_type": "GT_BATTLEGROUNDS",
        "build": 254477,
        "hero": "BG20_HERO_100",
        "local_player_id": 3,
        "final_place": 2,
        "start_health": {"3": 30, "5": 30, "6": 30},
        "shop": {"turns": [
            {"turn": 1, "tier": 1, "gold": 3, "rolls": 0, "buys": ["BG28_603t"]},
            {"turn": 2, "tier": 2, "gold": 4, "rolls": 2, "buys": []},
        ]},
        "rounds": [
            {"number": 1, "result": "won", "health_after": {"3": 30, "5": 27, "6": 0},
             "entries": [
                {"side": "own", "player_id": 3, "board": [minion(2, 3, &["taunt"])]},
                {"side": "opponent", "player_id": 5, "board": [minion(1, 1, &[])]},
             ]},
            {"number": 2, "result": "lost", "health_after": {"3": 25, "5": 27, "6": 0},
             "entries": [
                {"side": "own", "player_id": 3, "board": [
                    minion(4, 5, &["divine_shield", "taunt"]),
                    json!({"card_id": "BG28_603t", "atk": 1, "health": 1, "golden": true,
                           "position": 2, "keywords": ["deathrattle"]}),
                ]},
                {"side": "opponent", "player_id": 5, "board": [minion(6, 6, &[]), minion(1, 2, &[])]},
             ]},
        ],
    })
}

fn value(t: &Turn, name: &str) -> Option<f64> {
    let i = NAMES.iter().position(|n| *n == name).expect(name);
    t.values[i]
}

#[test]
fn every_feature_has_a_name_and_the_names_are_distinct() {
    let mut names = NAMES.to_vec();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), WIDTH);
}

#[test]
fn a_game_gives_one_row_per_combat_with_its_state() {
    let g = encode(&game()).unwrap();
    assert!(g.solo);
    assert_eq!((g.final_place, g.build), (2, Some(254_477)));
    assert_eq!(g.hero.as_deref(), Some("BG20_HERO_100"));
    assert_eq!(g.turns.len(), 2);
    let first = &g.turns[0];
    assert_eq!(value(first, "solo"), Some(1.0));
    assert_eq!(value(first, "turn"), Some(1.0));
    assert_eq!(value(first, "health"), Some(30.0));
    assert_eq!(value(first, "alive"), Some(3.0));
    assert_eq!(value(first, "health_rank"), Some(1.0));
    assert_eq!(value(first, "tavern_tier"), Some(1.0));
    assert_eq!(value(first, "buys"), Some(1.0));
    assert_eq!(value(first, "taunt"), Some(1.0));
    assert_eq!(value(first, "opponent_health"), Some(30.0));
    let second = &g.turns[1];
    // Health and the lobby after the first combat: seat 6 is out.
    assert_eq!(value(second, "health"), Some(30.0));
    assert_eq!(value(second, "alive"), Some(2.0));
    assert_eq!(value(second, "opponent_health"), Some(27.0));
    assert_eq!(value(second, "minions"), Some(2.0));
    assert_eq!(value(second, "attack_sum"), Some(5.0));
    assert_eq!(value(second, "health_sum"), Some(6.0));
    assert_eq!(value(second, "attack_max"), Some(4.0));
    assert_eq!(value(second, "golden"), Some(1.0));
    assert_eq!(value(second, "divine_shield"), Some(1.0));
    assert_eq!(value(second, "deathrattle"), Some(1.0));
    assert_eq!(value(second, "opponent_minions"), Some(2.0));
    assert_eq!(value(second, "opponent_attack_sum"), Some(7.0));
    assert_eq!(value(second, "rolls"), Some(2.0));
}

#[test]
fn what_the_record_does_not_have_is_unknown_never_zero() {
    let mut g = game();
    g.as_object_mut().unwrap().remove("shop");
    g.as_object_mut().unwrap().remove("start_health");
    g["rounds"][1]["entries"][1]["board"] = Value::Null;
    g["rounds"][1]["entries"][0]["board"][0]
        .as_object_mut()
        .unwrap()
        .remove("atk");
    let e = encode(&g).unwrap();
    let (first, second) = (&e.turns[0], &e.turns[1]);
    for name in [
        "health",
        "alive",
        "health_rank",
        "tavern_tier",
        "gold",
        "opponent_health",
    ] {
        assert_eq!(value(first, name), None, "{name}");
    }
    // A minion without its Attack: the sums are unknown, the count is not.
    assert_eq!(value(second, "minions"), Some(2.0));
    assert_eq!(value(second, "attack_sum"), None);
    assert_eq!(value(second, "health_sum"), None);
    assert_eq!(value(second, "opponent_minions"), None);
}

#[test]
fn a_combat_without_the_players_board_gives_no_row() {
    let mut g = game();
    g["rounds"][0]["entries"][0]["board"] = Value::Null;
    let e = encode(&g).unwrap();
    assert_eq!(e.turns.iter().map(|t| t.turn).collect::<Vec<_>>(), [2]);
}

#[test]
fn duos_reads_the_players_own_seat_and_places_up_to_four() {
    let mut g = game();
    g["game_type"] = json!("GT_BATTLEGROUNDS_DUO");
    // The teammate's board first: the player's own seat is still read.
    let teammate = json!({"side": "own", "player_id": 4, "board": [minion(9, 9, &[])]});
    g["rounds"][0]["entries"]
        .as_array_mut()
        .unwrap()
        .insert(0, teammate);
    let e = encode(&g).unwrap();
    assert!(!e.solo);
    assert_eq!(value(&e.turns[0], "attack_sum"), Some(2.0));
    g["final_place"] = json!(5);
    assert_eq!(encode(&g), Err(Skip::NoFinalPlace));
}

#[test]
fn records_that_are_not_games_are_skipped_with_a_reason() {
    assert_eq!(
        encode(&json!({"game_type": "GT_RANKED"})),
        Err(Skip::NotBattlegrounds)
    );
    let mut g = game();
    g.as_object_mut().unwrap().remove("final_place");
    assert_eq!(encode(&g), Err(Skip::NoFinalPlace));
    let mut g = game();
    g["rounds"] = json!([]);
    assert_eq!(encode(&g), Err(Skip::NoCombats));
    assert_eq!(encode(&json!("not a record")), Err(Skip::NotBattlegrounds));
}

#[test]
fn csv_rows_hold_numbers_card_ids_and_empty_cells_only() {
    let mut g = game();
    g["hero"] = json!("Someone, \"quoted\"");
    let e = encode(&g).unwrap();
    let header = csv_header();
    assert_eq!(header.split(',').count(), WIDTH + 5);
    let rows = csv_rows(7, &e);
    assert_eq!(rows.len(), 2);
    for row in &rows {
        assert_eq!(row.split(',').count(), WIDTH + 5, "{row}");
        assert!(
            row.starts_with("7,254477,,"),
            "a hero that is not a card id is left out: {row}"
        );
        assert!(row.ends_with(",2"), "{row}");
    }
}

#[test]
fn the_csv_header_names_each_column_once() {
    let header = csv_header();
    let mut cols: Vec<&str> = header.split(',').collect();
    let n = cols.len();
    cols.sort_unstable();
    cols.dedup();
    assert_eq!(cols.len(), n, "{header}");
}
