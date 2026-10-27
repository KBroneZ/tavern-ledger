//! How good the expected place is (T-653): cross-validation by game, so
//! the rows of one game are never on both sides, against two baselines
//! that know nothing of the position: the share of each place over all
//! games, and that share per turn. Every number is per game (each game
//! weighs the same), as the model is trained.

use std::collections::BTreeMap;

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

/// One calibration bucket: games whose expected place fell in
/// `from..from+1`, and the mean place they really got.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bucket {
    pub from: u32,
    pub rows: usize,
    pub said: f64,
    pub real: f64,
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
    let mut buckets: BTreeMap<u32, (usize, f64, f64)> = BTreeMap::new();
    for f in 0..folds {
        let train: Vec<Game> = mode
            .iter()
            .enumerate()
            .filter(|(i, _)| i % folds != f)
            .map(|(_, g)| (*g).clone())
            .collect();
        let test: Vec<&Game> = mode
            .iter()
            .enumerate()
            .filter(|(i, _)| i % folds == f)
            .map(|(_, g)| *g)
            .collect();
        let m = Model::train(&train, solo, opts)?;
        let train_refs: Vec<&Game> = train.iter().collect();
        let (all, by_turn) = baselines(&train_refs, k);
        for g in test {
            let w = 1.0 / g.turns.len() as f64;
            for t in &g.turns {
                let p = m.predict(t);
                model.add(&p, g.final_place, w);
                overall.add(&all, g.final_place, w);
                per_turn.add(by_turn.get(&t.turn).unwrap_or(&all), g.final_place, w);
                let said = expected(&p);
                let b = buckets.entry(said.floor() as u32).or_default();
                b.0 += 1;
                b.1 += said;
                b.2 += g.final_place as f64;
            }
        }
    }
    let calibration = buckets
        .into_iter()
        .map(|(from, (rows, said, real))| Bucket {
            from,
            rows,
            said: said / rows as f64,
            real: real / rows as f64,
        })
        .collect();
    Some(Report {
        solo,
        games: mode.len(),
        rows: mode.iter().map(|g| g.turns.len()).sum(),
        folds,
        model: model.score(),
        overall: overall.score(),
        per_turn: per_turn.score(),
        calibration,
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
