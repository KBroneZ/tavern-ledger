//! The website's calibration file (T-655): for each mode with a
//! cross-validated model, the held-out predictions grouped by what they
//! said, per game build, with the games and combats behind each group.
//! `place-train` writes it next to the models; the owner copies it to
//! `web/src/data/calibration.json` when a model is published.
//!
//! Only totals: no game, hero, player or time. A group's means are
//! published only with at least [`THIN_GAMES`] games behind them, and the
//! builds never overlap (no all-builds table beside them), so one build's
//! table cannot be subtracted from a total to bring back a smaller
//! build's groups. A limit on what is published, not a formal privacy
//! guarantee. Every prediction is held out: made by a model trained
//! without its game, so the numbers describe the training method on
//! unseen games rather than the published model itself.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{json, Value};

use crate::eval::{buckets, Report, Said};
use crate::model::{places, MIN_GAMES};
use crate::VERSION;

/// The file's format; the website refuses any other.
pub const FORMAT: u32 = 1;

/// The fewest games a build, or a group within it, needs before its means
/// and counts are published. Below it a group shows a dash and its counts
/// are left out too (null), so only "fewer than this" is told.
pub const THIN_GAMES: usize = 20;

/// The highest game build number taken as real; a build of 0, below 0 or
/// above it is counted with the games of no build.
pub const MAX_BUILD: i64 = 1_000_000_000;

/// The most builds listed per mode, newest first; the others are counted.
pub const MAX_BUILDS: usize = 20;

/// Rounded to two decimals, as the page shows them.
fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

fn distinct_games(predictions: &[&Said]) -> usize {
    predictions
        .iter()
        .map(|p| p.game)
        .collect::<BTreeSet<_>>()
        .len()
}

/// One build's every bucket, `1-2` to `places-1 - places`; an empty one
/// has no means and a thin one has neither means nor counts.
fn build_json(build: i64, predictions: &[&Said], k: usize) -> Value {
    let found = buckets(predictions.iter().copied(), k);
    let rows: Vec<Value> = (1..k as u32)
        .map(|from| {
            let b = found.iter().find(|b| b.from == from);
            let shown = b.filter(|b| b.games >= THIN_GAMES);
            let count = |n: usize| match b {
                None => json!(0),
                Some(_) if shown.is_none() => Value::Null,
                Some(_) => json!(n),
            };
            json!({
                "from": from,
                "to": from + 1,
                "rows": count(b.map_or(0, |b| b.rows)),
                "games": count(b.map_or(0, |b| b.games)),
                "said": shown.map(|b| round2(b.said)),
                "real": shown.map(|b| round2(b.real)),
            })
        })
        .collect();
    json!({
        "build": build,
        "games": distinct_games(predictions),
        "rows": predictions.len(),
        "buckets": rows,
    })
}

fn mode_json(r: &Report) -> Value {
    let k = places(r.solo);
    let mut by_build: BTreeMap<i64, Vec<&Said>> = BTreeMap::new();
    // A prediction that is not a number is in no bucket, and so in no
    // build's rows; a build number out of range counts as no build.
    for p in r.predictions.iter().filter(|p| p.is_finite()) {
        if let Some(b) = p.build.filter(|b| (1..=MAX_BUILD).contains(b)) {
            by_build.entry(b).or_default().push(p);
        }
    }
    let listed: Vec<(i64, Vec<&Said>)> = by_build
        .into_iter()
        .rev()
        .filter(|(_, preds)| distinct_games(preds) >= THIN_GAMES)
        .take(MAX_BUILDS)
        .collect();
    let shown_games: usize = listed.iter().map(|(_, p)| distinct_games(p)).sum();
    json!({
        "encoder": VERSION,
        "games": r.games,
        "rows": r.rows,
        "folds": r.folds,
        "min_games": MIN_GAMES,
        "ready": r.games >= MIN_GAMES,
        "thin_games": THIN_GAMES,
        "mean_distance": {
            "model": round2(r.model.mae),
            "per_turn": round2(r.per_turn.mae),
            "overall": round2(r.overall.mae),
        },
        "builds": listed.iter().map(|(b, p)| build_json(*b, p, k)).collect::<Vec<_>>(),
        "other_games": r.games - shown_games,
    })
}

/// The calibration file of a training run on `date` (`YYYY-MM-DD`): each
/// mode's cross-validation report, or `None` for a mode with no
/// cross-validated model. A mode with fewer than [`THIN_GAMES`] games is
/// left out too, since its mean distance would be over too few games.
/// With neither mode, the empty state (no date).
pub fn calibration_file(date: &str, solo: Option<&Report>, duos: Option<&Report>) -> Value {
    let enough = |r: &&Report| r.games >= THIN_GAMES;
    let (solo, duos) = (solo.filter(enough), duos.filter(enough));
    let date = if solo.is_none() && duos.is_none() {
        Value::Null
    } else {
        json!(date)
    };
    json!({
        "format": FORMAT,
        "date": date,
        "modes": {
            "solo": solo.map_or(Value::Null, mode_json),
            "duos": duos.map_or(Value::Null, mode_json),
        },
    })
}

/// A day counted from 1970-01-01 as `YYYY-MM-DD` (Howard Hinnant's
/// public-domain `civil_from_days` algorithm).
pub fn day_text(days: i64) -> String {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
#[path = "calibration_tests.rs"]
mod tests;
