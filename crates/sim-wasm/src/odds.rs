//! Runs a request the way the app's overlay does (`tracker::odds::run_odds`):
//! the same trials, batches and seeds, the same damage spread after the cap,
//! so the same boards give the same odds on the website and in the app. The
//! parity test (`tests/parity.rs`) checks it on fixed boards.

use std::collections::BTreeMap;

use combat_sim::cards::{has_start_of_combat, unchecked_build};
use combat_sim::{gifts, simulate_teams, Odds, Options, Verdict};
use serde_json::{json, Value};

use crate::input::Request;

/// The app's trials, batch size and seed (`tracker::odds`).
pub const TRIALS: u32 = 10_000;
const BATCH: u32 = 1_000;
const SEED: u64 = 0x0DD5;
/// The spread's ends: the 10th and 90th percentiles.
const LOW: f64 = 0.10;
const HIGH: f64 = 0.90;

/// The engine's version, shown with every answer.
pub const ENGINE: &str = combat_sim::VERSION;

/// The damage spread after the cap, or None when there is none to give.
fn spread(counts: &BTreeMap<i32, u32>, cap: Option<i64>) -> Option<Value> {
    let capped = |d: i32| cap.map_or(d, |c| d.min(i32::try_from(c).unwrap_or(i32::MAX)));
    let total: u32 = counts.values().sum();
    if total == 0 {
        return None;
    }
    let at = |share: f64| {
        let want = (share * f64::from(total)).ceil().max(1.0) as u32;
        let mut seen = 0;
        counts
            .iter()
            .find(|(_, n)| {
                seen += **n;
                seen >= want
            })
            .map_or(0, |(d, _)| capped(*d))
    };
    Some(json!({ "low": at(LOW), "likely": at(0.5), "high": at(HIGH) }))
}

pub(crate) fn answer(status: &str, fields: Value) -> Value {
    let mut out = json!({ "engine": ENGINE, "status": status });
    if let (Some(o), Value::Object(extra)) = (out.as_object_mut(), fields) {
        o.extend(extra);
    }
    out
}

/// A refusal before any combat is played.
pub fn refused(reason: &str) -> Value {
    answer("bad_input", json!({ "error": reason }))
}

/// All batches, merged; or the engine's verdict when it refuses.
fn play(request: &Request) -> Result<Odds, Value> {
    let mut total: Option<Odds> = None;
    for batch in 0..u64::from(TRIALS / BATCH) {
        let options = Options {
            trials: BATCH,
            seed: SEED ^ batch,
            start_of_combat: false,
            allow_unmodelled: false,
        };
        match simulate_teams(&request.player, &request.opponent, &options) {
            Ok(Verdict::Odds(o)) => match total.as_mut() {
                Some(t) => t.merge(&o),
                None => total = Some(o),
            },
            Ok(Verdict::NotModelled(c)) => {
                let approximate: Vec<Value> = c
                    .approximate
                    .iter()
                    .map(|(id, why)| json!({ "card_id": id, "reason": why }))
                    .collect();
                let reasons: Vec<Value> = c
                    .reasons()
                    .iter()
                    .map(|(id, why)| json!({ "card_id": id, "reason": why }))
                    .collect();
                return Err(answer(
                    "not_modelled",
                    json!({
                        "not_modelled": c.not_modelled,
                        "reasons": reasons,
                        "approximate": approximate,
                    }),
                ));
            }
            Err(e) => return Err(answer("bad_board", json!({ "error": e.to_string() }))),
        }
    }
    total.ok_or_else(|| answer("bad_board", json!({ "error": "no trials" })))
}

/// The odds for a request, as the page shows them.
pub fn run(request: &Request) -> Value {
    if !request.missing.is_empty() {
        return answer("no_card_data", json!({ "missing": request.missing }));
    }
    let odds = match play(request) {
        Ok(o) => o,
        Err(verdict) => return verdict,
    };
    let start_of_combat = request
        .player
        .boards()
        .chain(request.opponent.boards())
        .flat_map(|b| &b.minions)
        .any(|m| {
            has_start_of_combat(&m.card_id) || m.gifts.iter().any(|g| gifts::has_start_of_combat(g))
        });
    let known = odds.damage_unknown == 0;
    let cap = request.damage_cap;
    let approximate: Vec<Value> = odds
        .coverage
        .approximate
        .iter()
        .map(|(id, why)| json!({ "card_id": id, "reason": why }))
        .collect();
    answer(
        "odds",
        json!({
            "trials": odds.trials,
            "won": odds.won,
            "tied": odds.tied,
            "lost": odds.lost,
            "stalled": odds.stalled,
            "damage_known": known,
            "dealt": spread(&odds.damage_dealt, cap).filter(|_| known),
            "taken": spread(&odds.damage_taken, cap).filter(|_| known),
            "approximate": approximate,
            "start_of_combat": start_of_combat,
            "new_build": unchecked_build(request.build),
        }),
    )
}
