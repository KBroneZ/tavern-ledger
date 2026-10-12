//! Why each combat of a saved game went as it did (T-346, D-127), for the
//! recap: the simulator's odds on the boards as they fought, the real result
//! and, when the most likely result did not happen, the cause with its
//! evidence. The engine works it out (`combat_sim::saved`); the answer is the
//! same JSON the website's game page reads (`combat_sim::why_json`), so the
//! app and the site say the same.
//!
//! Card facts come from the app's card data, as for the overlay's odds; a
//! card the data lacks gives that combat no odds, by name.

use combat_sim::saved::{explain_game, CardFacts};
use combat_sim::why_json::why_json;
use combat_sim::Options;
use serde_json::Value;

use crate::odds::Facts;

/// Every combat with a result of `report` (a saved game's report), as JSON.
/// Runs the simulator on each combat (10 000 trials, fixed seed): call it
/// off the window's thread.
pub fn combat_whys(report: &Value, facts: &dyn Fn(&str) -> Option<Facts>) -> Value {
    let card_facts = |id: &str| {
        facts(id).map(|f| CardFacts {
            tier: f.tier,
            tribes: f.tribes,
            base: Some(f.base),
        })
    };
    why_json(&explain_game(report, &card_facts, &Options::default()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// A card with no combat text in the engine.
    const PLAIN: &str = "BG28_603t";

    fn facts(id: &str) -> Option<Facts> {
        (id == PLAIN).then(|| Facts::from_names(1, 1, Some(1), &[], &[]))
    }

    fn minion(atk: i64, health: i64, position: i64) -> Value {
        json!({"card_id": PLAIN, "golden": false, "atk": atk, "health": health,
               "position": position, "keywords": []})
    }

    #[test]
    fn every_combat_with_a_result_gets_its_odds_and_cause() {
        let entries = json!([
            {"side": "own", "player_id": 1, "board": [minion(3, 3, 1)]},
            {"side": "opponent", "player_id": 2, "board": [minion(1, 1, 1)]},
        ]);
        let report = json!({"game_type": "GT_BATTLEGROUNDS", "rounds": [
            {"number": 1, "result": "won", "entries": entries},
            {"number": 2, "result": "lost", "entries": entries},
            {"number": 3, "entries": entries},
        ]});
        let v = combat_whys(&report, &facts);
        let combats = v.as_array().expect("a list");
        assert_eq!(combats.len(), 2, "a combat without a result is left out");
        assert_eq!(combats[0]["cause"]["id"], "matched");
        assert_eq!(combats[0]["odds"]["won"], 1.0);
        assert_eq!(combats[1]["cause"]["id"], "no_replay_exact");
    }

    #[test]
    fn a_card_the_card_data_lacks_gives_no_odds_by_name() {
        let entries = json!([
            {"side": "own", "player_id": 1, "board": [{"card_id": "BG_UNKNOWN", "golden": false,
             "atk": 1, "health": 1, "position": 1, "keywords": []}]},
            {"side": "opponent", "player_id": 2, "board": [minion(1, 1, 1)]},
        ]);
        let report = json!({"game_type": "GT_BATTLEGROUNDS",
                            "rounds": [{"number": 1, "result": "won", "entries": entries}]});
        let v = combat_whys(&report, &facts);
        assert_eq!(v[0]["no_odds"], "no_card_data");
        assert_eq!(v[0]["cards"], json!(["BG_UNKNOWN"]));
    }
}
