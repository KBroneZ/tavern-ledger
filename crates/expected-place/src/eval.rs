//! How good the expected place is (T-653): cross-validation by game, so
//! the rows of one game are never on both sides, against two baselines
//! that know nothing of the position: the share of each place over all
//! games, and that share per turn. Every number is per game (each game
//! weighs the same), as the model is trained.

use std::collections::{BTreeMap, BTreeSet};

use crate::model::{expected, places, Model, Train};
use crate::Game;

/// Scores of one way of guessing places.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Score {
    /// Mean negative log-likelihood of the real place (lower is better).
    pub log_loss: f64,
    /// Mean distance between the expected place and the real one.
    pub mae: f64,
    /// Mean ranked probability score: like log-loss but a near miss (2nd
    /// for a 1st) costs less than a far one, since places are in order.
    pub rps: f64,
}

/// One calibration bucket: the held-out combats whose expected place fell
/// in `from..from+1` (a sure last place in the last bucket), the games
/// they come from, and the mean place said and really got (each combat
/// once).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bucket {
    pub from: u32,
    pub rows: usize,
    pub games: usize,
    pub said: f64,
    pub real: f64,
}

/// One held-out prediction: the game it is from (its index among the
/// mode's games, in the order given; never an id from the record), the
/// game's build, the expected place said and the place really got.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Said {
    pub game: usize,
    pub build: Option<i64>,
    pub said: f64,
    pub place: i64,
}

/// The bucket an expected place falls in among the `places - 1` buckets
/// `1..2` to `places-1..places`; `None` for a value that is not a number
/// (never put in a bucket, see [`Said::is_finite`]).
pub fn bucket_of(said: f64, places: usize) -> Option<u32> {
    if !said.is_finite() {
        return None;
    }
    let last = places.saturating_sub(1).max(1) as f64;
    Some(said.floor().clamp(1.0, last) as u32)
}

impl Said {
    /// Whether the expected place is a number. A model always gives one;
    /// `place-train` stops with an error if one is not, and the buckets and
    /// the calibration file leave such a prediction out entirely.
    pub fn is_finite(&self) -> bool {
        self.said.is_finite()
    }
}

/// The non-empty buckets of `predictions`, lowest first; a prediction that
/// is not a number is left out (neither a row nor a game).
pub fn buckets<'a>(predictions: impl IntoIterator<Item = &'a Said>, places: usize) -> Vec<Bucket> {
    let mut sums: BTreeMap<u32, (usize, BTreeSet<usize>, f64, f64)> = BTreeMap::new();
    for p in predictions {
        let Some(from) = bucket_of(p.said, places) else {
            continue;
        };
        let b = sums.entry(from).or_default();
        b.0 += 1;
        b.1.insert(p.game);
        b.2 += p.said;
        b.3 += p.place as f64;
    }
    sums.into_iter()
        .map(|(from, (rows, games, said, real))| Bucket {
            from,
            rows,
            games: games.len(),
            said: said / rows as f64,
            real: real / rows as f64,
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    pub solo: bool,
    pub games: usize,
    pub rows: usize,
    pub folds: usize,
    pub model: Score,
    /// The share of each place over the training games.
    pub overall: Score,
    /// The share of each place among training rows of the same turn.
    pub per_turn: Score,
    pub calibration: Vec<Bucket>,
    /// Every held-out prediction, in fold order. Kept in memory only: the
    /// calibration file publishes totals of at least
    /// [`crate::calibration::THIN_GAMES`] games.
    pub predictions: Vec<Said>,
}

/// A place distribution from weighted counts, smoothed so no place is 0.
fn distribution(counts: &[f64]) -> Vec<f64> {
    let total: f64 = counts.iter().sum::<f64>() + counts.len() as f64;
    counts.iter().map(|c| (c + 1.0) / total).collect()
}

/// The place shares of `games` (each weighing 1), over all rows and per turn.
fn baselines(games: &[&Game], k: usize) -> (Vec<f64>, BTreeMap<i64, Vec<f64>>) {
    let mut all = vec![0.0; k];
    let mut by_turn: BTreeMap<i64, Vec<f64>> = BTreeMap::new();
    for g in games {
        let label = (g.final_place - 1) as usize;
        all[label] += 1.0;
        let w = 1.0 / g.turns.len() as f64;
        for t in &g.turns {
            by_turn.entry(t.turn).or_insert_with(|| vec![0.0; k])[label] += w;
        }
    }
    let by_turn = by_turn
        .into_iter()
        .map(|(t, c)| (t, distribution(&c)))
        .collect();
    (distribution(&all), by_turn)
}

#[derive(Default)]
struct Sum {
    loss: f64,
    mae: f64,
    rps: f64,
    weight: f64,
}

/// The ranked probability score of `p` for the real place, over the
/// cumulative shares (0 is a perfect, sure guess).
fn rps(p: &[f64], place: i64) -> f64 {
    let mut said = 0.0;
    let mut out = 0.0;
    for (k, q) in p.iter().enumerate().take(p.len() - 1) {
        said += q;
        let real = if (k as i64) + 1 >= place { 1.0 } else { 0.0 };
        out += (said - real).powi(2);
    }
    out / (p.len() - 1) as f64
}

impl Sum {
    fn add(&mut self, p: &[f64], place: i64, w: f64) {
        let real = p[(place - 1) as usize].max(1e-12);
        self.loss += -w * real.ln();
        self.mae += w * (expected(p) - place as f64).abs();
        self.rps += w * rps(p, place);
        self.weight += w;
    }

    fn score(&self) -> Score {
        if self.weight == 0.0 {
            return Score::default();
        }
        Score {
            log_loss: self.loss / self.weight,
            mae: self.mae / self.weight,
            rps: self.rps / self.weight,
        }
    }
}

/// Cross-validates the model of one mode over `games` in `folds` folds by
/// game (game `i` goes to fold `i % folds`, in the order given). `None`
/// with fewer games of that mode than folds.
pub fn cross_validate(games: &[Game], solo: bool, folds: usize, opts: &Train) -> Option<Report> {
    let mode: Vec<&Game> = games.iter().filter(|g| g.solo == solo).collect();
    if folds < 2 || mode.len() < folds {
        return None;
    }
    let k = places(solo);
    let (mut model, mut overall, mut per_turn) = (Sum::default(), Sum::default(), Sum::default());
    let mut predictions = Vec::new();
    for f in 0..folds {
        let train: Vec<Game> = mode
            .iter()
            .enumerate()
            .filter(|(i, _)| i % folds != f)
            .map(|(_, g)| (*g).clone())
            .collect();
        let test: Vec<(usize, &Game)> = mode
            .iter()
            .enumerate()
            .filter(|(i, _)| i % folds == f)
            .map(|(i, g)| (i, *g))
            .collect();
        let m = Model::train(&train, solo, opts)?;
        let train_refs: Vec<&Game> = train.iter().collect();
        let (all, by_turn) = baselines(&train_refs, k);
        for (i, g) in test {
            let w = 1.0 / g.turns.len() as f64;
            for t in &g.turns {
                let p = m.predict(t);
                model.add(&p, g.final_place, w);
                overall.add(&all, g.final_place, w);
                per_turn.add(by_turn.get(&t.turn).unwrap_or(&all), g.final_place, w);
                predictions.push(Said {
                    game: i,
                    build: g.build,
                    said: expected(&p),
                    place: g.final_place,
                });
            }
        }
    }
    let calibration = buckets(&predictions, k);
    Some(Report {
        solo,
        games: mode.len(),
        rows: mode.iter().map(|g| g.turns.len()).sum(),
        folds,
        model: model.score(),
        overall: overall.score(),
        per_turn: per_turn.score(),
        calibration,
        predictions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rps_is_zero_for_a_sure_right_guess_and_worse_the_farther_the_miss() {
        assert_eq!(rps(&[1.0, 0.0, 0.0, 0.0], 1), 0.0);
        let near = rps(&[0.0, 1.0, 0.0, 0.0], 1);
        let far = rps(&[0.0, 0.0, 0.0, 1.0], 1);
        assert!(near > 0.0 && far > near, "{near} {far}");
    }
}
