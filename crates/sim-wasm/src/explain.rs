//! Why each combat of a saved game went as it did (T-549, D-127): the game
//! page sends the game's report as stored and the card facts of its cards;
//! the answer is the same JSON the app's recap reads
//! (`combat_sim::why_json`), so the two say the same.
//!
//! Request: `{"explain": {"report": {...}, "facts": {"<card id>": facts | null}}}`
//! with the facts as [`crate::input`] reads them. Answer:
//! `{"engine": "0.8.0", "status": "explained", "combats": [...]}`, or a
//! refusal (`bad_input`) with a short reason. Untrusted input: the report is
//! read defensively by the engine, the facts strictly here.

use std::collections::HashMap;

use combat_sim::saved::{explain_game, CardFacts};
use combat_sim::why_json::why_json;
use combat_sim::Options;
use serde_json::{json, Map, Value};

use crate::input::{facts, valid_id};
use crate::odds::{answer, refused};

/// Far above a real request: upload-game's cap on an inflated record is
/// 1 MiB, and the facts of its cards are a few kilobytes.
pub const MAX_EXPLAIN_BYTES: usize = 2 * 1024 * 1024;
/// Distinct cards in one game: far fewer in practice.
const MAX_CARDS: usize = 2_000;

/// The request's `explain` object, if the request is one.
pub fn explain_of(v: &Value) -> Option<&Value> {
    v.as_object()?.get("explain")
}

fn card_facts(v: Option<&Value>) -> Result<HashMap<String, CardFacts>, String> {
    let map: &Map<String, Value> = v
        .and_then(Value::as_object)
        .filter(|m| m.len() <= MAX_CARDS)
        .ok_or("facts is not a map of cards")?;
    let mut out = HashMap::new();
    for (id, f) in map {
        if !valid_id(id) {
            return Err("facts holds a bad card id".into());
        }
        if let Some(f) = facts(f)? {
            let facts = CardFacts {
                tier: f.tier,
                tribes: f.tribes,
                base: Some(f.base),
            };
            out.insert(id.clone(), facts);
        }
    }
    Ok(out)
}

/// The answer to an `explain` request.
pub fn run(explain: &Value) -> Value {
    let Some(report) = explain.get("report").filter(|r| r.is_object()) else {
        return refused("the report is not an object");
    };
    let known = match card_facts(explain.get("facts")) {
        Ok(k) => k,
        Err(reason) => return refused(&reason),
    };
    let whys = explain_game(report, &|id| known.get(id).copied(), &Options::default());
    answer("explained", json!({ "combats": why_json(&whys) }))
}
