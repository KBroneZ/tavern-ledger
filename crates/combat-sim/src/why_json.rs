//! Why each combat went as it did, as the JSON the app's recap and the
//! website's game page read (T-346, T-549): one shape for both, so the two
//! say the same. Card ids, numbers, the cause id ([`Cause::id`]) and its
//! evidence; nothing about the players.
//!
//! ```text
//! [{"round": 4, "result": "lost",
//!   "odds": {"won": 0.62, "tied": 0.08, "lost": 0.30} | null,
//!   "no_odds": null | "no_boards" | "no_card_data" | "not_modelled" | "bad_board",
//!   "cards": [card ids without data, or not modelled],
//!   "cause": null | {"id": "missing_inputs", "attacks": 9, "attack": 4,
//!                    "rule": "replay_stats", "card": "BG…",
//!                    "inputs": [{"what": "blood_gems", "card": "BG…"}],
//!                    "cards": ["BG…"], "replay": "not_saved"}}]
//! ```
//!
//! A cause carries only the evidence its id has: `attacks` for chance;
//! `attack`, `rule` and `card` where the replay parts; `inputs` for
//! missing inputs; `cards` for an approximate card; `replay` without one.

use serde_json::{json, Map, Value};

use crate::combat::replay::Difference;
use crate::combat::CombatResult;
use crate::saved::{CombatWhy, Why};
use crate::why::{rule_id, Cause, NoReplay, Said, Unseen};

fn result_word(r: CombatResult) -> &'static str {
    match r {
        CombatResult::Won => "won",
        CombatResult::Lost => "lost",
        CombatResult::Tie => "tie",
    }
}

fn unseen_word(u: Unseen) -> &'static str {
    match u {
        Unseen::BloodGems => "blood_gems",
        Unseen::Spells => "spells",
        Unseen::Knights => "knights",
        Unseen::Beetles => "beetles",
        Unseen::Hand => "hand",
        Unseen::Earlier => "earlier",
        Unseen::Gift => "gift",
    }
}

fn no_replay_word(n: NoReplay) -> &'static str {
    match n {
        NoReplay::NotSaved => "not_saved",
        NoReplay::Unreadable => "unreadable",
        NoReplay::Stops => "stops",
    }
}

/// Where the replay parts: the attack, the rule's id and the card.
fn parted(out: &mut Map<String, Value>, d: &Difference) {
    out.insert("attack".into(), json!(d.attack));
    out.insert("rule".into(), json!(rule_id(d.rule)));
    if let Some(card) = &d.card {
        out.insert("card".into(), json!(card));
    }
}

fn cause_json(c: &Cause) -> Value {
    let mut out = Map::new();
    out.insert("id".into(), json!(c.id()));
    match c {
        Cause::Matched => {}
        Cause::Chance { attacks } => {
            out.insert("attacks".into(), json!(attacks));
        }
        Cause::MissingInputs { inputs, parted: d } => {
            parted(&mut out, d);
            let inputs: Vec<Value> = inputs
                .iter()
                .map(|n| json!({"what": unseen_word(n.what), "card": n.card}))
                .collect();
            out.insert("inputs".into(), Value::Array(inputs));
        }
        Cause::ApproximateCard { cards, parted: d } => {
            parted(&mut out, d);
            out.insert("cards".into(), json!(cards));
        }
        Cause::RuleDifference(d) => parted(&mut out, d),
        Cause::NoReplay { why, .. } => {
            out.insert("replay".into(), json!(no_replay_word(*why)));
        }
    }
    Value::Object(out)
}

fn combat_json(c: &CombatWhy) -> Value {
    let (odds, no_odds, cards, cause) = match &c.why {
        Why::NoBoards => (Value::Null, json!("no_boards"), Vec::new(), Value::Null),
        Why::NoCardData(ids) => (Value::Null, json!("no_card_data"), ids.clone(), Value::Null),
        Why::Explained(e) => match &e.said {
            Said::Odds(p) => (
                json!({"won": p.won, "tied": p.tied, "lost": p.lost}),
                Value::Null,
                Vec::new(),
                e.cause.as_ref().map_or(Value::Null, cause_json),
            ),
            Said::NotModelled(ids) => {
                (Value::Null, json!("not_modelled"), ids.clone(), Value::Null)
            }
            Said::BadBoard => (Value::Null, json!("bad_board"), Vec::new(), Value::Null),
        },
    };
    // Why each card blocks the odds, for a card the engine refuses.
    let reasons: Vec<Value> = if no_odds == json!("not_modelled") {
        cards
            .iter()
            .map(|id| json!({"card_id": id, "reason": crate::trials::refusal_reason(id)}))
            .collect()
    } else {
        Vec::new()
    };
    json!({
        "round": c.round,
        "result": result_word(c.result),
        "odds": odds,
        "no_odds": no_odds,
        "cards": cards,
        "reasons": reasons,
        "cause": cause,
    })
}

/// Every explained combat of a game, in round order.
pub fn why_json(whys: &[CombatWhy]) -> Value {
    Value::Array(whys.iter().map(combat_json).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::replay::Rule;
    use crate::score::Prediction;
    use crate::why::{Cards, Explained, NotSeen};

    fn odds(cause: Cause) -> Why {
        Why::Explained(Explained {
            said: Said::Odds(Prediction {
                won: 0.75,
                tied: 0.0,
                lost: 0.25,
            }),
            cause: Some(cause),
        })
    }

    fn one(why: Why) -> Value {
        let c = CombatWhy {
            round: 4,
            result: CombatResult::Lost,
            why,
        };
        why_json(&[c])[0].clone()
    }

    #[test]
    fn a_miss_carries_its_id_and_where_the_replay_parts() {
        let d = Difference {
            attack: 3,
            rule: Rule::Stats,
            detail: "player position 1".into(),
            card: Some("BG_X".into()),
        };
        let v = one(odds(Cause::MissingInputs {
            inputs: vec![NotSeen {
                what: Unseen::BloodGems,
                card: "BG_GEM".into(),
            }],
            parted: d.clone(),
        }));
        assert_eq!(v["round"], 4);
        assert_eq!(v["result"], "lost");
        assert_eq!(v["odds"]["won"], 0.75);
        assert_eq!(v["no_odds"], Value::Null);
        let c = &v["cause"];
        assert_eq!(c["id"], "missing_inputs");
        assert_eq!(
            (c["attack"].clone(), c["rule"].clone()),
            (json!(3), json!("replay_stats"))
        );
        assert_eq!(c["card"], "BG_X");
        assert_eq!(
            c["inputs"],
            json!([{"what": "blood_gems", "card": "BG_GEM"}])
        );
        // The detail (positions) stays in the harness.
        assert!(c.get("detail").is_none());
        let v = one(odds(Cause::RuleDifference(d)));
        assert_eq!(v["cause"]["id"], "replay_stats");
        assert!(v["cause"].get("inputs").is_none());
    }

    #[test]
    fn chance_matched_and_no_replay_carry_only_their_evidence() {
        let v = one(odds(Cause::Chance { attacks: 9 }));
        assert_eq!(v["cause"], json!({"id": "chance", "attacks": 9}));
        assert_eq!(one(odds(Cause::Matched))["cause"], json!({"id": "matched"}));
        let v = one(odds(Cause::NoReplay {
            why: NoReplay::NotSaved,
            cards: Cards::Exact,
        }));
        assert_eq!(
            v["cause"],
            json!({"id": "no_replay_exact", "replay": "not_saved"})
        );
    }

    #[test]
    fn no_odds_says_why_and_names_the_cards() {
        let v = one(Why::NoCardData(vec!["ZZ_1".into()]));
        assert_eq!(v["odds"], Value::Null);
        assert_eq!(v["no_odds"], "no_card_data");
        assert_eq!(v["cards"], json!(["ZZ_1"]));
        assert_eq!(v["cause"], Value::Null);
        assert_eq!(one(Why::NoBoards)["no_odds"], "no_boards");
        let v = one(Why::Explained(Explained {
            said: Said::NotModelled(vec!["ZZ_2".into()]),
            cause: None,
        }));
        assert_eq!(
            (v["no_odds"].clone(), v["cards"].clone()),
            (json!("not_modelled"), json!(["ZZ_2"]))
        );
        assert_eq!(v["reasons"][0]["card_id"], "ZZ_2");
        assert!(v["reasons"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("not reviewed"));
    }

    #[test]
    fn a_refused_card_gives_its_own_reason() {
        let v = one(Why::Explained(Explained {
            said: Said::NotModelled(vec!["BG31_999".into()]),
            cause: None,
        }));
        let reason = v["reasons"][0]["reason"].as_str().unwrap();
        assert!(reason.contains("copies the minion it destroys"), "{reason}");
        assert_eq!(one(Why::NoBoards)["reasons"], json!([]));
    }
}
