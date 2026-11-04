//! The curve on made-up Duos games where health decides the place.

use super::*;
use crate::model::{Train, MIN_GAMES};
use crate::NAMES;
use crate::WIDTH;

fn index(name: &str) -> usize {
    NAMES.iter().position(|n| *n == name).unwrap()
}

/// A turn whose only known numbers are the turn, the player's health and
/// the opponent's board Attack (which never decides a place here).
fn turn(t: i64, health: f64) -> Turn {
    let mut values = [None; WIDTH];
    values[index("solo")] = Some(0.0);
    values[index("turn")] = Some(t as f64);
    values[index("health")] = Some(health);
    values[index("opponent_attack_sum")] = Some(10.0 + t as f64);
    Turn { turn: t, values }
}

/// A Duos game whose player's health tells the final place: the healthier,
/// the better (place 1 near 40, place 4 near 10).
fn game(place: i64, turns: i64) -> Game {
    let health = 50.0 - 10.0 * place as f64;
    Game {
        solo: false,
        build: Some(1),
        hero: None,
        final_place: place,
        turns: (1..=turns).map(|t| turn(t, health - t as f64)).collect(),
    }
}

/// A model trained on enough made-up games to be shown.
fn ready_model() -> Model {
    let games: Vec<Game> = (0..MIN_GAMES as i64)
        .map(|i| game(1 + i % 4, 3 + i % 5))
        .collect();
    let m = Model::train(&games, false, &Train::default()).unwrap();
    assert!(m.ready());
    m
}

/// A game whose health holds, then falls hard on turn 3.
fn falling_game() -> Game {
    Game {
        turns: vec![turn(1, 39.0), turn(2, 38.0), turn(3, 12.0), turn(4, 11.0)],
        ..game(4, 0)
    }
}

fn dashes(c: &Curve) -> Option<Dash> {
    assert!(c.points.iter().all(|p| p.shown.is_none()) && c.worst.is_none());
    c.dash
}

#[test]
fn without_a_model_every_turn_is_a_dash() {
    let c = curve(None, &falling_game());
    assert_eq!(
        c.points.iter().map(|p| p.turn).collect::<Vec<_>>(),
        [1, 2, 3, 4]
    );
    assert_eq!(dashes(&c), Some(Dash::NoModel));
}

#[test]
fn a_model_on_too_few_games_shows_dashes() {
    let mut m = ready_model();
    m.games = MIN_GAMES - 1;
    assert_eq!(
        dashes(&curve(Some(&m), &falling_game())),
        Some(Dash::TooFewGames)
    );
}

#[test]
fn a_model_of_the_other_mode_shows_dashes() {
    let m = ready_model();
    let solo = Game {
        solo: true,
        ..falling_game()
    };
    assert_eq!(dashes(&curve(Some(&m), &solo)), Some(Dash::OtherMode));
}

#[test]
fn a_model_of_another_feature_version_shows_dashes() {
    let mut m = ready_model();
    m.encoder = VERSION + 1;
    assert_eq!(
        dashes(&curve(Some(&m), &falling_game())),
        Some(Dash::OtherFormat)
    );
}

#[test]
fn each_turn_shows_the_expected_place_and_a_band_around_it() {
    let m = ready_model();
    let c = curve(Some(&m), &falling_game());
    for (p, t) in c.points.iter().zip(&falling_game().turns) {
        let s = p.shown.as_ref().expect("a ready model shows every turn");
        let probs = m.predict(t);
        assert!((s.mean - expected(&probs)).abs() < 1e-12);
        assert!(1 <= s.low && s.low <= s.high && s.high <= 4, "{s:?}");
        assert!(
            (s.low as f64) <= s.mean + 1.0 && s.mean - 1.0 <= s.high as f64,
            "{s:?}"
        );
    }
    let mean = |i: usize| c.points[i].shown.as_ref().unwrap().mean;
    assert!(mean(0) < 2.0 && mean(3) > 3.0, "{c:?}");
}

#[test]
fn the_turn_the_place_fell_most_is_marked_with_its_reason() {
    let m = ready_model();
    let c = curve(Some(&m), &falling_game());
    let worst = c.worst.expect("health fell hard on turn 3");
    assert_eq!((worst.index, worst.turn), (2, 3));
    assert_eq!(c.dash, None);
    assert_eq!(worst.reason, Some("health"));
}

#[test]
fn a_steady_game_marks_no_turn() {
    let m = ready_model();
    let steady = Game {
        turns: vec![turn(1, 39.0), turn(2, 39.0), turn(3, 39.0)],
        ..game(1, 0)
    };
    assert_eq!(curve(Some(&m), &steady).worst, None);
}

#[test]
fn the_first_turn_is_never_marked() {
    let m = ready_model();
    let one = Game {
        turns: vec![turn(1, 11.0)],
        ..game(4, 0)
    };
    let c = curve(Some(&m), &one);
    assert!(c.points[0].shown.is_some());
    assert_eq!(c.worst, None);
}

#[test]
fn a_game_without_turns_gives_an_empty_curve() {
    let m = ready_model();
    let c = curve(Some(&m), &game(2, 0));
    assert!(c.points.is_empty() && c.worst.is_none());
}

#[test]
fn the_band_is_the_middle_half_of_the_places() {
    assert_eq!(band(&[1.0, 0.0, 0.0, 0.0]), (1, 1));
    assert_eq!(band(&[0.25, 0.25, 0.25, 0.25]), (1, 3));
    assert_eq!(band(&[0.1, 0.1, 0.1, 0.7]), (3, 4));
    assert_eq!(band(&[0.0, 0.5, 0.5, 0.0]), (2, 3));
}

#[test]
fn a_model_of_the_wrong_shape_shows_dashes_instead_of_panicking() {
    let shapes: [fn(&mut Model); 4] = [
        |m| m.mean.pop().map(drop).unwrap(),
        |m| m.spread.push(1.0),
        |m| m.intercepts.pop().map(drop).unwrap(),
        |m| m.weights[1].pop().map(drop).unwrap(),
    ];
    for change in shapes {
        let mut m = ready_model();
        change(&mut m);
        assert_eq!(
            dashes(&curve(Some(&m), &falling_game())),
            Some(Dash::OtherFormat)
        );
    }
}

#[test]
fn a_turn_that_is_not_a_number_is_a_dash_and_breaks_the_rise() {
    let m = ready_model();
    let mut g = falling_game();
    g.turns[1].values[index("health")] = Some(f64::NAN);
    g.turns[2].values[index("health")] = Some(f64::INFINITY);
    let c = curve(Some(&m), &g);
    assert!(c.points[0].shown.is_some() && c.points[3].shown.is_some());
    assert!(c.points[1].shown.is_none() && c.points[2].shown.is_none());
    assert_eq!(c.worst, None, "no rise across the gap");
}

#[test]
fn no_reason_when_no_feature_takes_back_half_of_the_rise() {
    let m = ready_model();
    let g = falling_game();
    let now = curve(Some(&m), &g).points[2].shown.clone().unwrap().mean;
    let (before, after) = (&g.turns[1], &g.turns[2]);
    let rise = now - curve(Some(&m), &g).points[1].shown.clone().unwrap().mean;
    assert_eq!(reason(&m, before, after, now, rise), Some("health"));
    // Health takes back the whole rise, so not half of three times it.
    assert_eq!(reason(&m, before, after, now, 3.0 * rise), None);
}

#[test]
fn the_features_never_given_as_a_reason_exist() {
    for name in NOT_A_REASON {
        assert!(NAMES.contains(&name), "{name}");
    }
}

#[test]
fn a_solo_model_shows_eight_places_on_a_solo_game() {
    let solo = |place: i64, turns: i64| {
        let mut g = game(place, turns);
        g.solo = true;
        for t in &mut g.turns {
            t.values[index("solo")] = Some(1.0);
            t.values[index("health")] = Some(45.0 - 5.0 * place as f64 - t.turn as f64);
        }
        g
    };
    let games: Vec<Game> = (0..MIN_GAMES as i64)
        .map(|i| solo(1 + i % 8, 3 + i % 5))
        .collect();
    let m = Model::train(&games, true, &Train::default()).unwrap();
    let c = curve(Some(&m), &solo(8, 4));
    assert_eq!(c.dash, None);
    for p in &c.points {
        let s = p.shown.as_ref().unwrap();
        assert!(1 <= s.low && s.low <= s.high && s.high <= 8, "{s:?}");
    }
    assert!(c.points[0].shown.as_ref().unwrap().mean > 5.0, "{c:?}");
}

#[test]
fn the_json_has_dashes_points_and_the_worst_turn() {
    let m = ready_model();
    let v = to_json(&curve(Some(&m), &falling_game()));
    assert_eq!(v["dash"], serde_json::Value::Null);
    assert_eq!(v["points"].as_array().unwrap().len(), 4);
    assert!(v["points"][0]["mean"].as_f64().is_some());
    assert_eq!(v["worst"]["turn"], 3);
    assert_eq!(v["worst"]["reason"], "health");
    let none = to_json(&curve(None, &falling_game()));
    assert_eq!(none["dash"], "no_model");
    assert_eq!(
        none["points"][1],
        serde_json::json!({"turn": 2, "mean": null})
    );
    assert_eq!(none["worst"], serde_json::Value::Null);
}
