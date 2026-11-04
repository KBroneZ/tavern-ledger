//! The first expected-place model (T-653): for each mode, a multinomial
//! logistic regression over the final places, trained on the rows of
//! [`crate::encode`]. Small on purpose: it predicts in microseconds on a
//! player's PC or in the browser, and the same code trains and predicts.
//!
//! Inputs: each feature standardised with its training mean and spread,
//! and beside it a 0/1 mark that it is unknown (an unknown value then
//! counts as the mean). Each game weighs the same however many combats it
//! had, so long games do not outweigh short ones. Training is full-batch
//! gradient descent from zero weights with a fixed number of steps:
//! the same rows always give the same model.

use serde_json::{json, Value};

use crate::{Game, Turn, VERSION, WIDTH};

/// The model file's format; a reader refuses any other.
pub const FORMAT: u32 = 1;

/// The fewest games of a mode a model must be trained on before players
/// see its numbers; below it a turn shows a dash. A starting point, not a
/// proof: the model card's cross-validation must also beat the baselines.
pub const MIN_GAMES: usize = 150;

/// Inputs per row: each feature and its unknown mark.
pub const INPUTS: usize = 2 * WIDTH;

/// How the model is trained.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Train {
    pub steps: u32,
    pub rate: f64,
    /// L2 penalty on the weights (not the intercepts).
    pub l2: f64,
}

impl Default for Train {
    fn default() -> Self {
        Train {
            steps: 400,
            rate: 0.5,
            l2: 0.01,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Model {
    pub solo: bool,
    /// The rows' format it was trained on ([`crate::VERSION`]).
    pub encoder: u32,
    pub mean: Vec<f64>,
    pub spread: Vec<f64>,
    /// One row of [`INPUTS`] weights per place, best place first.
    pub weights: Vec<Vec<f64>>,
    pub intercepts: Vec<f64>,
    /// What it was trained on.
    pub games: usize,
    pub rows: usize,
}

/// Places in a mode.
pub fn places(solo: bool) -> usize {
    if solo {
        8
    } else {
        4
    }
}

/// The mean and spread of each feature over its known values, each row
/// weighted.
fn scale(rows: &[(&Turn, f64)]) -> (Vec<f64>, Vec<f64>) {
    let mut mean = vec![0.0; WIDTH];
    let mut spread = vec![1.0; WIDTH];
    for f in 0..WIDTH {
        let known: Vec<(f64, f64)> = rows
            .iter()
            .filter_map(|(t, w)| t.values[f].map(|v| (v, *w)))
            .collect();
        let total: f64 = known.iter().map(|(_, w)| w).sum();
        if total <= 0.0 {
            continue;
        }
        let m = known.iter().map(|(v, w)| v * w).sum::<f64>() / total;
        let var = known.iter().map(|(v, w)| w * (v - m).powi(2)).sum::<f64>() / total;
        mean[f] = m;
        // A feature that never varies gives no signal: spread 1 keeps it at 0.
        spread[f] = if var > 1e-12 { var.sqrt() } else { 1.0 };
    }
    (mean, spread)
}

fn inputs(t: &Turn, mean: &[f64], spread: &[f64]) -> Vec<f64> {
    let mut x = vec![0.0; INPUTS];
    for f in 0..WIDTH {
        match t.values[f] {
            Some(v) => x[f] = (v - mean[f]) / spread[f],
            None => x[WIDTH + f] = 1.0,
        }
    }
    x
}

fn softmax(z: &[f64]) -> Vec<f64> {
    let top = z.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let e: Vec<f64> = z.iter().map(|v| (v - top).exp()).collect();
    let sum: f64 = e.iter().sum();
    e.iter().map(|v| v / sum).collect()
}

/// The chance of each place (best first) and its mean, the expected place.
pub fn expected(p: &[f64]) -> f64 {
    p.iter().enumerate().map(|(k, q)| (k + 1) as f64 * q).sum()
}

impl Model {
    /// Trains a model on the games of one mode (others are ignored).
    /// `None` without a game of that mode.
    pub fn train(games: &[Game], solo: bool, opts: &Train) -> Option<Model> {
        let games: Vec<&Game> = games.iter().filter(|g| g.solo == solo).collect();
        let k = places(solo);
        // Each game weighs 1 in all, shared by its rows.
        let rows: Vec<(&Turn, f64, usize)> = games
            .iter()
            .flat_map(|g| {
                let w = 1.0 / g.turns.len() as f64;
                let label = usize::try_from(g.final_place - 1).unwrap_or(0).min(k - 1);
                g.turns.iter().map(move |t| (t, w, label))
            })
            .collect();
        if rows.is_empty() {
            return None;
        }
        let weighted: Vec<(&Turn, f64)> = rows.iter().map(|(t, w, _)| (*t, *w)).collect();
        let (mean, spread) = scale(&weighted);
        let xs: Vec<Vec<f64>> = rows
            .iter()
            .map(|(t, _, _)| inputs(t, &mean, &spread))
            .collect();
        let total: f64 = rows.iter().map(|(_, w, _)| w).sum();
        let mut weights = vec![vec![0.0; INPUTS]; k];
        let mut intercepts = vec![0.0; k];
        for _ in 0..opts.steps {
            let mut gw = vec![vec![0.0; INPUTS]; k];
            let mut gb = vec![0.0; k];
            for (x, (_, w, label)) in xs.iter().zip(&rows) {
                let z: Vec<f64> = (0..k)
                    .map(|c| intercepts[c] + dot(&weights[c], x))
                    .collect();
                let p = softmax(&z);
                for c in 0..k {
                    let err = w * (p[c] - if c == *label { 1.0 } else { 0.0 });
                    gb[c] += err;
                    for (g, xi) in gw[c].iter_mut().zip(x) {
                        *g += err * xi;
                    }
                }
            }
            for c in 0..k {
                intercepts[c] -= opts.rate * gb[c] / total;
                for (wi, g) in weights[c].iter_mut().zip(&gw[c]) {
                    *wi -= opts.rate * (g / total + opts.l2 * *wi);
                }
            }
        }
        Some(Model {
            solo,
            encoder: VERSION,
            mean,
            spread,
            weights,
            intercepts,
            games: games.len(),
            rows: rows.len(),
        })
    }

    /// Whether players may see this model's numbers (enough games).
    pub fn ready(&self) -> bool {
        self.games >= MIN_GAMES
    }

    /// The chance of each place for one row, best place first.
    pub fn predict(&self, t: &Turn) -> Vec<f64> {
        let x = inputs(t, &self.mean, &self.spread);
        let z: Vec<f64> = self
            .weights
            .iter()
            .zip(&self.intercepts)
            .map(|(w, b)| b + dot(w, &x))
            .collect();
        softmax(&z)
    }

    pub fn to_json(&self) -> Value {
        json!({
            "format": FORMAT,
            "encoder": self.encoder,
            "mode": if self.solo { "solo" } else { "duos" },
            "games": self.games,
            "rows": self.rows,
            "mean": self.mean,
            "spread": self.spread,
            "weights": self.weights,
            "intercepts": self.intercepts,
        })
    }

    /// A model file, checked: format, encoder version, sizes, finite numbers.
    pub fn from_json(v: &Value) -> Result<Model, String> {
        if v.get("format").and_then(Value::as_u64) != Some(u64::from(FORMAT)) {
            return Err(format!("model format is not {FORMAT}"));
        }
        if v.get("encoder").and_then(Value::as_u64) != Some(u64::from(VERSION)) {
            return Err(format!(
                "the model was trained on rows of another format than {VERSION}"
            ));
        }
        let solo = match v.get("mode").and_then(Value::as_str) {
            Some("solo") => true,
            Some("duos") => false,
            _ => return Err("mode is not solo or duos".into()),
        };
        let k = places(solo);
        let nums = |key: &str, n: usize| -> Result<Vec<f64>, String> {
            let list = v
                .get(key)
                .and_then(Value::as_array)
                .ok_or(format!("{key} is missing"))?;
            let out: Vec<f64> = list
                .iter()
                .filter_map(Value::as_f64)
                .filter(|x| x.is_finite())
                .collect();
            (out.len() == n && list.len() == n)
                .then_some(out)
                .ok_or(format!("{key} is not {n} numbers"))
        };
        let weights: Vec<Vec<f64>> = v
            .get("weights")
            .and_then(Value::as_array)
            .filter(|rows| rows.len() == k)
            .ok_or(format!("weights is not {k} rows"))?
            .iter()
            .map(|row| {
                let out: Vec<f64> = row
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_f64)
                    .filter(|x| x.is_finite())
                    .collect();
                (out.len() == INPUTS)
                    .then_some(out)
                    .ok_or(format!("a weights row is not {INPUTS} numbers"))
            })
            .collect::<Result<_, _>>()?;
        let spread = nums("spread", WIDTH)?;
        if spread.iter().any(|s| *s <= 0.0) {
            return Err("a spread is not above 0".into());
        }
        let size = |key: &str| {
            v.get(key)
                .and_then(Value::as_u64)
                .map(|n| n as usize)
                .ok_or(format!("{key} is missing"))
        };
        Ok(Model {
            solo,
            encoder: VERSION,
            mean: nums("mean", WIDTH)?,
            spread,
            weights,
            intercepts: nums("intercepts", k)?,
            games: size("games")?,
            rows: size("rows")?,
        })
    }
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

#[cfg(test)]
#[path = "model_tests.rs"]
mod tests;
