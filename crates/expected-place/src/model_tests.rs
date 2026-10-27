//! The model and its cross-validation on made-up games where health
//! decides the place.

use super::*;
use crate::eval::cross_validate;
use crate::NAMES;
use serde_json::json;

fn index(name: &str) -> usize {
    NAMES.iter().position(|n| *n == name).unwrap()
}

/// A Duos game of `turns` combats whose player's health tells the final
/// place: the healthier, the better (place 1 at 40, place 4 at 10).
fn game(place: i64, turns: i64) -> Game {
    let health = 50.0 - 10.0 * place as f64;
    Game {
        solo: false,
        build: Some(1),
        hero: None,
        final_place: place,
        turns: (1..=turns)
            .map(|t| {
                let mut values = [None; WIDTH];
                values[index("turn")] = Some(t as f64);
                values[index("health")] = Some(health - t as f64);
                // Never known: must not upset anything.
                values[index("gold")] = None;
                Turn { turn: t, values }
            })
            .collect(),
    }
}

fn games() -> Vec<Game> {
    (0..40).map(|i| game(1 + i % 4, 3 + i % 5)).collect()
}

#[test]
fn the_model_learns_that_health_tells_the_place() {
    let m = Model::train(&games(), false, &Train::default()).unwrap();
    assert_eq!(
        (m.games, m.weights.len(), m.weights[0].len()),
        (40, 4, INPUTS)
    );
    let best = m.predict(&game(1, 3).turns[1]);
    let worst = m.predict(&game(4, 3).turns[1]);
    assert!((best.iter().sum::<f64>() - 1.0).abs() < 1e-9);
    assert!(expected(&best) < 1.6, "{best:?}");
    assert!(expected(&worst) > 3.4, "{worst:?}");
}

#[test]
fn the_same_games_give_the_same_model() {
    let a = Model::train(&games(), false, &Train::default()).unwrap();
    let b = Model::train(&games(), false, &Train::default()).unwrap();
    assert_eq!(a, b);
}

#[test]
fn a_mode_without_games_gives_no_model() {
    assert!(Model::train(&games(), true, &Train::default()).is_none());
    assert_eq!(places(true), 8);
}

#[test]
fn an_unknown_value_reads_as_the_mean_with_its_mark() {
    let m = Model::train(&games(), false, &Train::default()).unwrap();
    let mut t = game(2, 3).turns[0].clone();
    t.values[index("health")] = None;
    let p = m.predict(&t);
    assert!(p.iter().all(|q| q.is_finite() && *q > 0.0));
}

#[test]
fn the_model_file_reads_back_and_refuses_what_it_should() {
    let m = Model::train(&games(), false, &Train::default()).unwrap();
    let v = m.to_json();
    assert_eq!(Model::from_json(&v).unwrap(), m);
    let broken = |edit: &dyn Fn(&mut Value)| {
        let mut w = v.clone();
        edit(&mut w);
        Model::from_json(&w).is_err()
    };
    assert!(broken(&|w| w["format"] = json!(9)));
    assert!(broken(&|w| w["encoder"] = json!(VERSION + 1)));
    assert!(broken(&|w| w["mode"] = json!("trio")));
    assert!(broken(&|w| w["mean"] = json!([1, 2])));
    assert!(broken(&|w| w["spread"][0] = json!(0)));
    assert!(broken(&|w| w["weights"][0][0] = json!("x")));
    assert!(broken(&|w| w["weights"] = json!([[0.0]])));
    assert!(broken(&|w| w["intercepts"] = json!([0, 0])));
}

#[test]
fn cross_validation_beats_the_baselines_when_the_signal_is_there() {
    let r = cross_validate(&games(), false, 5, &Train::default()).unwrap();
    assert_eq!((r.games, r.folds), (40, 5));
    assert!(r.model.log_loss < r.overall.log_loss, "{r:?}");
    assert!(r.model.mae < r.per_turn.mae, "{r:?}");
    assert!(r.calibration.iter().all(|b| b.rows > 0));
    assert!(r.model.rps < r.overall.rps, "{r:?}");
    assert!(cross_validate(&games()[..3], false, 5, &Train::default()).is_none());
}

#[test]
fn a_model_on_few_games_is_not_ready_for_players() {
    let m = Model::train(&games(), false, &Train::default()).unwrap();
    assert!(!m.ready());
    let many: Vec<Game> = (0..MIN_GAMES as i64).map(|i| game(1 + i % 4, 3)).collect();
    assert!(Model::train(
        &many,
        false,
        &Train {
            steps: 1,
            ..Train::default()
        }
    )
    .unwrap()
    .ready());
}
