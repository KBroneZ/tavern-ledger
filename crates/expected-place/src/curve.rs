//! The expected-place curve (T-654): per combat of a game, the place the
//! model expects and the middle half of the places it gives, the turn the
//! expected place rose most (a worse place is a higher number) and what
//! changed most with it. The app's recap and the website's game page draw
//! it beside the health curve.
//!
//! Every turn is a dash (`None`) while no model may be shown: none, one
//! trained on fewer than [`crate::model::MIN_GAMES`] games, one of the other mode, or
//! one trained on rows of another format. The middle half is the spread of
//! the places the model gives that turn, not how sure the model is.
//!
//! The reason is the feature whose change since the previous turn, put
//! back alone, takes back most of the rise: what changed with it, not a
//! proven cause (turn, tier and board stats move together). No reason is
//! given when no single feature takes back at least half of the rise.

use serde_json::{json, Value};

use crate::model::{expected, places, Model, INPUTS};
use crate::{Game, Turn, NAMES, VERSION, WIDTH};

/// A rise in expected place smaller than this is not marked (places).
pub const MARK_RISE: f64 = 0.25;

/// The share of the rise a feature must take back to be the reason.
pub const REASON_SHARE: f64 = 0.5;

/// Features never given as a reason: they change every turn or never.
const NOT_A_REASON: [&str; 2] = ["solo", "turn"];

/// One combat's point of the curve.
#[derive(Clone, Debug, PartialEq)]
pub struct Point {
    /// The game's turn of this combat.
    pub turn: i64,
    /// `None`: a dash.
    pub shown: Option<Shown>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Shown {
    /// The expected place (1 is best).
    pub mean: f64,
    /// The middle half of the places the model gives: the 25th and 75th
    /// percentile places.
    pub low: usize,
    pub high: usize,
}

/// The turn whose expected place rose most since the combat before it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Worst {
    /// Index in [`Curve::points`].
    pub index: usize,
    /// Its game turn.
    pub turn: i64,
    /// A feature name of [`NAMES`]; the app and website put it in words.
    pub reason: Option<&'static str>,
}

/// Why every turn is a dash.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dash {
    NoModel,
    /// Trained on fewer than [`crate::model::MIN_GAMES`] games.
    TooFewGames,
    /// A Solo model for a Duos game or the reverse.
    OtherMode,
    /// Trained on rows of another format, or not of the expected shape.
    OtherFormat,
}

impl Dash {
    pub fn id(self) -> &'static str {
        match self {
            Dash::NoModel => "no_model",
            Dash::TooFewGames => "too_few_games",
            Dash::OtherMode => "other_mode",
            Dash::OtherFormat => "other_format",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Curve {
    pub points: Vec<Point>,
    pub worst: Option<Worst>,
    /// `Some` when every turn is a dash, with why.
    pub dash: Option<Dash>,
}

/// Whether players may see this model's numbers for this game; why not.
fn shows<'m>(m: Option<&'m Model>, game: &Game) -> Result<&'m Model, Dash> {
    let m = m.ok_or(Dash::NoModel)?;
    let k = places(game.solo);
    if m.solo != game.solo {
        Err(Dash::OtherMode)
    } else if m.encoder != VERSION
        || m.mean.len() != WIDTH
        || m.spread.len() != WIDTH
        || m.weights.len() != k
        || m.intercepts.len() != k
        || m.weights.iter().any(|w| w.len() != INPUTS)
    {
        Err(Dash::OtherFormat)
    } else if !m.ready() {
        Err(Dash::TooFewGames)
    } else {
        Ok(m)
    }
}

/// The chance of each place for a row; `None` for anything not a number.
fn predict(m: &Model, t: &Turn) -> Option<Vec<f64>> {
    let p = m.predict(t);
    p.iter().all(|q| q.is_finite()).then_some(p)
}

/// The 25th and 75th percentile places (1-based) of a distribution over
/// places, best first.
#[must_use]
pub fn band(p: &[f64]) -> (usize, usize) {
    let at = |q: f64| {
        let mut sum = 0.0;
        for (k, x) in p.iter().enumerate() {
            sum += x;
            if sum >= q - 1e-12 {
                return k + 1;
            }
        }
        p.len().max(1)
    };
    (at(0.25), at(0.75))
}

fn point(m: Option<&Model>, t: &Turn) -> Point {
    let shown = m.and_then(|m| {
        let p = predict(m, t)?;
        let (low, high) = band(&p);
        Some(Shown {
            mean: expected(&p),
            low,
            high,
        })
    });
    Point {
        turn: t.turn,
        shown,
    }
}

/// The feature that, put back to its previous value alone, takes back at
/// least [`REASON_SHARE`] of the rise and the most of it (`now_mean` is
/// the later turn's expected place).
fn reason(m: &Model, before: &Turn, now: &Turn, now_mean: f64, rise: f64) -> Option<&'static str> {
    let mut best: Option<(f64, &'static str)> = None;
    for (f, name) in NAMES.iter().enumerate() {
        if NOT_A_REASON.contains(name) || before.values[f] == now.values[f] {
            continue;
        }
        let mut back = now.clone();
        back.values[f] = before.values[f];
        let Some(taken) = predict(m, &back).map(|p| now_mean - expected(&p)) else {
            continue;
        };
        if taken > 0.0 && best.is_none_or(|(b, _)| taken > b) {
            best = Some((taken, name));
        }
    }
    best.filter(|(taken, _)| *taken >= REASON_SHARE * rise)
        .map(|(_, name)| name)
}

/// The curve of a game's combats, in the game's order (one combat per
/// turn; a rise is measured from the combat before, whatever its turn).
#[must_use]
pub fn curve(model: Option<&Model>, game: &Game) -> Curve {
    let (model, dash) = match shows(model, game) {
        Ok(m) => (Some(m), None),
        Err(d) => (None, Some(d)),
    };
    let points: Vec<Point> = game.turns.iter().map(|t| point(model, t)).collect();
    let mut worst: Option<(f64, f64, usize)> = None;
    for i in 1..points.len() {
        let (Some(a), Some(b)) = (&points[i - 1].shown, &points[i].shown) else {
            continue;
        };
        let rise = b.mean - a.mean;
        if rise >= MARK_RISE && worst.is_none_or(|(r, _, _)| rise > r) {
            worst = Some((rise, b.mean, i));
        }
    }
    let worst = worst.zip(model).map(|((rise, mean, index), m)| Worst {
        index,
        turn: points[index].turn,
        reason: reason(m, &game.turns[index - 1], &game.turns[index], mean, rise),
    });
    Curve {
        points,
        worst,
        dash,
    }
}

/// The curve for the app and the website: `{"dash": null | why,
/// "points": [{"turn", "mean", "low", "high"} | {"turn", "mean": null}],
/// "worst": null | {"index", "turn", "reason"}}`.
#[must_use]
pub fn to_json(c: &Curve) -> Value {
    let points: Vec<Value> = c
        .points
        .iter()
        .map(|p| match &p.shown {
            Some(s) => json!({"turn": p.turn, "mean": s.mean, "low": s.low, "high": s.high}),
            None => json!({"turn": p.turn, "mean": null}),
        })
        .collect();
    json!({
        "dash": c.dash.map(Dash::id),
        "points": points,
        "worst": c.worst.map(|w| json!({"index": w.index, "turn": w.turn, "reason": w.reason})),
    })
}

#[cfg(test)]
#[path = "curve_tests.rs"]
mod tests;
