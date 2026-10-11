//! Scoring predictions against real results (T-603): how much probability
//! the engine gave to what happened, and whether its numbers are calibrated
//! (of the times it said 30%, about 30% happened).

use crate::combat::CombatResult;

/// Predicted shares for one combat; they add up to 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Prediction {
    pub won: f64,
    pub tied: f64,
    pub lost: f64,
}

impl Prediction {
    pub fn of(&self, result: CombatResult) -> f64 {
        match result {
            CombatResult::Won => self.won,
            CombatResult::Tie => self.tied,
            CombatResult::Lost => self.lost,
        }
    }

    /// The most likely result (ties broken toward Won, then Tie).
    pub fn top(&self) -> CombatResult {
        if self.won >= self.tied && self.won >= self.lost {
            CombatResult::Won
        } else if self.tied >= self.lost {
            CombatResult::Tie
        } else {
            CombatResult::Lost
        }
    }
}

pub const RESULTS: [CombatResult; 3] = [CombatResult::Won, CombatResult::Tie, CombatResult::Lost];

/// Shares in `[lo, hi)` (the last bucket includes 1.0).
#[derive(Clone, Debug, PartialEq)]
pub struct Bucket {
    pub lo: f64,
    pub hi: f64,
    /// Predictions (one per combat and result) that fell here.
    pub count: u32,
    pub mean_predicted: f64,
    /// Share of those that happened.
    pub observed: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    pub combats: u32,
    /// Mean probability given to the result that happened.
    pub mean_p_actual: f64,
    /// Multi-class Brier score (0 is perfect, 2 is worst).
    pub brier: f64,
    /// Combats where the most likely result is the one that happened.
    pub top_pick_hits: u32,
    pub buckets: Vec<Bucket>,
}

pub fn score(items: &[(Prediction, CombatResult)], buckets: usize) -> Report {
    let buckets = buckets.max(1);
    let n = items.len() as f64;
    let mut sums = vec![(0u32, 0.0f64, 0u32); buckets];
    let mut p_actual = 0.0;
    let mut brier = 0.0;
    let mut hits = 0;
    for (pred, actual) in items {
        p_actual += pred.of(*actual);
        if pred.top() == *actual {
            hits += 1;
        }
        for r in RESULTS {
            let p = pred.of(r);
            let happened = r == *actual;
            brier += (p - if happened { 1.0 } else { 0.0 }).powi(2);
            let b = ((p * buckets as f64) as usize).min(buckets - 1);
            sums[b].0 += 1;
            sums[b].1 += p;
            sums[b].2 += u32::from(happened);
        }
    }
    let width = 1.0 / buckets as f64;
    Report {
        combats: items.len() as u32,
        mean_p_actual: if n > 0.0 { p_actual / n } else { 0.0 },
        brier: if n > 0.0 { brier / n } else { 0.0 },
        top_pick_hits: hits,
        buckets: sums
            .iter()
            .enumerate()
            .map(|(i, &(count, p, happened))| Bucket {
                lo: i as f64 * width,
                hi: (i + 1) as f64 * width,
                count,
                mean_predicted: if count > 0 { p / f64::from(count) } else { 0.0 },
                observed: if count > 0 {
                    f64::from(happened) / f64::from(count)
                } else {
                    0.0
                },
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(won: f64, tied: f64, lost: f64) -> Prediction {
        Prediction { won, tied, lost }
    }

    #[test]
    fn a_sure_and_right_prediction_scores_perfectly() {
        let r = score(&[(p(1.0, 0.0, 0.0), CombatResult::Won)], 10);
        assert_eq!(r.mean_p_actual, 1.0);
        assert_eq!(r.brier, 0.0);
        assert_eq!(r.top_pick_hits, 1);
        assert_eq!(r.buckets[9].count, 1);
        assert_eq!(r.buckets[9].observed, 1.0);
        assert_eq!(r.buckets[0].count, 2);
        assert_eq!(r.buckets[0].observed, 0.0);
    }

    #[test]
    fn a_sure_and_wrong_prediction_scores_worst() {
        let r = score(&[(p(0.0, 0.0, 1.0), CombatResult::Won)], 10);
        assert_eq!(r.mean_p_actual, 0.0);
        assert_eq!(r.brier, 2.0);
        assert_eq!(r.top_pick_hits, 0);
    }

    #[test]
    fn calibration_buckets_compare_said_with_happened() {
        let items = [
            (p(0.5, 0.0, 0.5), CombatResult::Won),
            (p(0.5, 0.0, 0.5), CombatResult::Lost),
        ];
        let r = score(&items, 2);
        // Four predictions of 0.5 (bucket 1), half happened.
        assert_eq!(r.buckets[1].count, 4);
        assert_eq!(r.buckets[1].observed, 0.5);
        assert!((r.mean_p_actual - 0.5).abs() < 1e-12);
    }

    #[test]
    fn no_items_no_nan() {
        let r = score(&[], 10);
        assert_eq!(r.combats, 0);
        assert_eq!(r.mean_p_actual, 0.0);
    }
}
