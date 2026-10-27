//! The calibration file (T-655) on made-up held-out predictions: what it
//! holds, what it leaves out, and that no published mean has fewer than
//! `THIN_GAMES` games behind it.

use super::*;
use crate::eval::{Said, Score};
use serde_json::json;

/// A made-up game: its build and its rows as (expected place said, real
/// place).
type Fake = (Option<i64>, Vec<(f64, i64)>);

/// Held-out predictions of `games`.
fn report(solo: bool, games: &[Fake]) -> Report {
    let predictions: Vec<Said> = games
        .iter()
        .enumerate()
        .flat_map(|(game, (build, rows))| {
            rows.iter().map(move |&(said, place)| Said {
                game,
                build: *build,
                said,
                place,
            })
        })
        .collect();
    let score = |mae| Score {
        log_loss: 1.0,
        mae,
        rps: 0.1,
    };
    Report {
        solo,
        games: games.len(),
        rows: predictions.len(),
        folds: 5,
        model: score(0.81234),
        overall: score(1.24567),
        per_turn: score(1.10001),
        calibration: Vec::new(),
        predictions,
    }
}

/// Duos: build 200 with 30 games (1.5 said for a 1st and 2.5 for a 3rd
/// each), build 100 with 25 games (3.3 for a 4th each, and the first 10
/// also 1.2 for a 2nd), build 50 with 5 games and 3 games without a build.
fn duos() -> Report {
    let mut games = Vec::new();
    for _ in 0..30 {
        games.push((Some(200), vec![(1.5, 1), (2.5, 3)]));
    }
    for i in 0..25 {
        let mut rows = vec![(3.3, 4)];
        if i < 10 {
            rows.push((1.2, 2));
        }
        games.push((Some(100), rows));
    }
    for _ in 0..5 {
        games.push((Some(50), vec![(2.0, 2)]));
    }
    for _ in 0..3 {
        games.push((None, vec![(2.0, 2)]));
    }
    report(false, &games)
}

#[test]
fn with_no_model_the_file_is_the_empty_state() {
    assert_eq!(
        calibration_file("2026-10-27", None, None),
        json!({"format": FORMAT, "date": null, "modes": {"solo": null, "duos": null}})
    );
}

#[test]
fn a_mode_with_fewer_games_than_the_floor_is_not_published() {
    let few: Vec<_> = (0..THIN_GAMES - 1)
        .map(|_| (Some(1), vec![(2.0, 2)]))
        .collect();
    let v = calibration_file("2026-11-02", None, Some(&report(false, &few)));
    assert_eq!(v["modes"]["duos"], Value::Null);
    assert_eq!(v["date"], Value::Null);
}

#[test]
fn the_website_reads_the_format_this_crate_writes() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../web/src/data/calibration.json"
    );
    let text = std::fs::read_to_string(path).expect("web/src/data/calibration.json");
    let v: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v["format"], json!(FORMAT));
}

#[test]
fn a_mode_lists_its_builds_newest_first_with_every_bucket() {
    let v = calibration_file("2026-11-02", None, Some(&duos()));
    assert_eq!(v["date"], json!("2026-11-02"));
    assert_eq!(v["modes"]["solo"], Value::Null);
    let m = &v["modes"]["duos"];
    assert_eq!(m["encoder"], json!(crate::VERSION));
    assert_eq!(
        (m["games"].clone(), m["rows"].clone()),
        (json!(63), json!(103))
    );
    assert_eq!(m["folds"], json!(5));
    assert_eq!(m["min_games"], json!(crate::model::MIN_GAMES));
    assert_eq!(m["ready"], json!(false));
    assert_eq!(m["thin_games"], json!(THIN_GAMES));
    assert_eq!(
        m["mean_distance"],
        json!({"model": 0.81, "per_turn": 1.1, "overall": 1.25})
    );
    assert_eq!(m["other_games"], json!(8));
    let builds = m["builds"].as_array().unwrap();
    assert_eq!(builds.len(), 2);

    assert_eq!(
        builds[0],
        json!({"build": 200, "games": 30, "rows": 60, "buckets": [
            {"from": 1, "to": 2, "rows": 30, "games": 30, "said": 1.5, "real": 1.0},
            {"from": 2, "to": 3, "rows": 30, "games": 30, "said": 2.5, "real": 3.0},
            {"from": 3, "to": 4, "rows": 0, "games": 0, "said": null, "real": null},
        ]})
    );
    assert_eq!(
        builds[1],
        json!({"build": 100, "games": 25, "rows": 35, "buckets": [
            {"from": 1, "to": 2, "rows": 10, "games": 10, "said": null, "real": null},
            {"from": 2, "to": 3, "rows": 0, "games": 0, "said": null, "real": null},
            {"from": 3, "to": 4, "rows": 25, "games": 25, "said": 3.3, "real": 4.0},
        ]})
    );
}

#[test]
fn solo_has_seven_buckets_and_a_ready_model_says_so() {
    let games: Vec<_> = (0..crate::model::MIN_GAMES)
        .map(|i| (Some(7), vec![(1.0 + (i % 8) as f64, 1 + (i % 8) as i64)]))
        .collect();
    let sevens_and_eights = games.iter().filter(|g| g.1[0].0 >= 7.0).count();
    let v = calibration_file("2026-11-02", Some(&report(true, &games)), None);
    let m = &v["modes"]["solo"];
    assert_eq!(m["ready"], json!(true));
    let buckets = m["builds"][0]["buckets"].as_array().unwrap();
    let froms: Vec<i64> = buckets
        .iter()
        .map(|b| b["from"].as_i64().unwrap())
        .collect();
    assert_eq!(froms, (1..=7).collect::<Vec<_>>());
    // A sure 8th (8.0 said) falls in 7-8 with the 7.0s.
    assert_eq!(buckets[6]["rows"], json!(sevens_and_eights));
}

#[test]
fn at_most_twenty_builds_are_listed_and_the_rest_are_counted() {
    let games: Vec<_> = (0..22)
        .flat_map(|b| (0..THIN_GAMES).map(move |_| (Some(1000 + b), vec![(2.5, 2)])))
        .collect();
    let v = calibration_file("2026-11-02", None, Some(&report(false, &games)));
    let m = &v["modes"]["duos"];
    let builds = m["builds"].as_array().unwrap();
    assert_eq!(builds.len(), MAX_BUILDS);
    assert_eq!(builds[0]["build"], json!(1021));
    assert_eq!(m["other_games"], json!(2 * THIN_GAMES));
}

#[test]
fn no_published_mean_has_fewer_games_than_the_floor_and_no_game_is_named() {
    let v = calibration_file("2026-11-02", None, Some(&duos()));
    let text = serde_json::to_string(&v).unwrap();
    for word in ["\"game\"", "hero", "player", "name", "account", "place\""] {
        assert!(!text.contains(word), "{word}");
    }
    for b in v["modes"]["duos"]["builds"].as_array().unwrap() {
        assert!(b["games"].as_u64().unwrap() >= THIN_GAMES as u64);
        for bucket in b["buckets"].as_array().unwrap() {
            let thin = bucket["games"].as_u64().unwrap() < THIN_GAMES as u64;
            assert_eq!(bucket["said"].is_null(), thin, "{bucket}");
            assert_eq!(bucket["real"].is_null(), thin, "{bucket}");
        }
    }
}

#[test]
fn a_day_number_reads_as_its_calendar_day() {
    assert_eq!(day_text(0), "1970-01-01");
    assert_eq!(day_text(-1), "1969-12-31");
    assert_eq!(day_text(11_016), "2000-02-29");
    assert_eq!(day_text(20_753), "2026-10-27");
}
