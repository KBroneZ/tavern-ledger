//! Each combat's damage as the game dealt it (T-540, D-108): the winner's
//! hero hits the loser's as the combat ends, and the saved combat replay
//! (parser revision 8, D-099) keeps that hit (`hero_damage`) with its amount.
//! That is the game's own number, so it replaces the health rule (D-057:
//! health before minus after, which also counts health paid or healed in the
//! shop) wherever the record has it. A tie the game recorded (D-055) hit
//! nobody. Anything else is not in the record: the caller falls back to the
//! health rule and marks it inferred.
//!
//! In Duos the hit lands on the team, which shares one health.

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

use crate::recap::own_health_points;

/// No hero attack in the game comes near this; a larger number is a
/// hand-edited record, not a hit.
const MAX_HIT: i64 = 1_000;

/// One combat's damage from the log: what your hero (your team in Duos)
/// took and dealt. One of them is 0; both are 0 after a tie.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct CombatDamage {
    pub round: i64,
    pub taken: i64,
    pub dealt: i64,
}

/// The damage of one saved round, when the record has the game's own hit
/// (or a tie the game recorded); None otherwise.
pub fn logged_damage(round: &Value) -> Option<CombatDamage> {
    let number = round.get("number")?.as_i64()?;
    let tie = round.get("result").and_then(Value::as_str) == Some("tie");
    let hit = round
        .get("replay")
        .and_then(|r| r.get("events"))
        .and_then(Value::as_array)
        .and_then(|events| {
            events
                .iter()
                .rev()
                .find(|e| e.get("kind").and_then(Value::as_str) == Some("hero_damage"))
        });
    let (taken, dealt) = match hit {
        Some(e) => {
            let amount = e
                .get("amount")?
                .as_i64()
                .filter(|a| (1..=MAX_HIT).contains(a))?;
            match e.get("side")?.as_str()? {
                "own" => (amount, 0),
                "opponent" => (0, amount),
                _ => return None,
            }
        }
        None if tie => (0, 0),
        None => return None,
    };
    Some(CombatDamage {
        round: number,
        taken,
        dealt,
    })
}

/// Every combat of a report whose damage the record has, in round order.
pub fn game_damage(report: &Value) -> Vec<CombatDamage> {
    let by_round: BTreeMap<i64, CombatDamage> = report
        .get("rounds")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(logged_damage)
        .map(|d| (d.round, d))
        .collect();
    by_round.into_values().collect()
}

/// The damage your hero took in one combat and where it comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Taken {
    pub amount: i64,
    pub from_log: bool,
}

/// The damage taken in each combat of a game: the game's own hit where the
/// record has it, else health before minus after (inferred, never below 0);
/// a combat with neither is left out.
pub fn damage_taken(report: &Value) -> Vec<Taken> {
    let logged: BTreeMap<i64, i64> = game_damage(report)
        .into_iter()
        .map(|d| (d.round, d.taken))
        .collect();
    let health: BTreeMap<i64, Option<i64>> = own_health_points(report)
        .into_iter()
        .map(|p| (p.round, p.health))
        .collect();
    let rounds = health.keys().copied().filter(|&r| r > 0);
    rounds
        .chain(logged.keys().copied())
        .collect::<std::collections::BTreeSet<i64>>()
        .into_iter()
        .filter_map(|round| match logged.get(&round) {
            Some(&amount) => Some(Taken {
                amount,
                from_log: true,
            }),
            None => {
                let before = (*health.get(&(round - 1))?)?;
                let after = (*health.get(&round)?)?;
                Some(Taken {
                    amount: (before - after).max(0),
                    from_log: false,
                })
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn hit(side: &str, amount: i64) -> Value {
        json!({"kind": "hero_damage", "side": side, "amount": amount})
    }

    fn round(n: i64, result: Option<&str>, events: Option<Vec<Value>>) -> Value {
        let mut r = json!({"number": n, "result": result, "entries": []});
        if let Some(events) = events {
            r["replay"] = json!({"start": 0, "complete": true, "minions": [], "events": events});
        }
        r
    }

    #[test]
    fn the_games_hit_gives_the_damage_taken_or_dealt() {
        let lost = round(
            3,
            Some("lost"),
            Some(vec![json!({"kind": "attack"}), hit("own", 7)]),
        );
        assert_eq!(
            logged_damage(&lost),
            Some(CombatDamage {
                round: 3,
                taken: 7,
                dealt: 0
            })
        );
        let won = round(4, Some("won"), Some(vec![hit("opponent", 12)]));
        assert_eq!(
            logged_damage(&won).map(|d| (d.taken, d.dealt)),
            Some((0, 12))
        );
    }

    #[test]
    fn a_tie_the_game_recorded_hit_nobody() {
        let tie = round(2, Some("tie"), Some(vec![json!({"kind": "attack"})]));
        assert_eq!(
            logged_damage(&tie).map(|d| (d.taken, d.dealt)),
            Some((0, 0))
        );
        // An older record with the game's tie but no replay: still no hit.
        let old_tie = round(2, Some("tie"), None);
        assert_eq!(logged_damage(&old_tie).map(|d| d.taken), Some(0));
    }

    #[test]
    fn without_the_hit_the_damage_is_not_in_the_record() {
        // Won or lost, but the replay lacks the hit (cut short): unknown.
        assert_eq!(logged_damage(&round(1, Some("won"), Some(vec![]))), None);
        // An older record: no replay at all.
        assert_eq!(logged_damage(&round(1, Some("lost"), None)), None);
        // A replay the game did not play back.
        let mut hidden = round(1, Some("lost"), None);
        hidden["replay"] = Value::Null;
        assert_eq!(logged_damage(&hidden), None);
    }

    #[test]
    fn hand_edited_hits_are_not_used() {
        for bad in [
            json!({"kind": "hero_damage", "side": "own", "amount": "7"}),
            json!({"kind": "hero_damage", "side": "own", "amount": -3}),
            json!({"kind": "hero_damage", "side": "own", "amount": 0}),
            json!({"kind": "hero_damage", "side": "own", "amount": 5000}),
            json!({"kind": "hero_damage", "side": "nobody", "amount": 3}),
            json!({"kind": "hero_damage", "amount": 3}),
        ] {
            assert_eq!(
                logged_damage(&round(1, Some("lost"), Some(vec![bad]))),
                None
            );
        }
        assert_eq!(logged_damage(&json!({"number": "1"})), None);
        assert!(game_damage(&json!({"rounds": "no"})).is_empty());
    }

    /// A Solo game, we are seat 1, starting at 30 health.
    fn game(rounds: Vec<Value>) -> Value {
        json!({
            "game_type": "GT_BATTLEGROUNDS", "local_player_id": 1, "hero": "H",
            "lobby": [{"player_id": 1, "hero": "H"}, {"player_id": 2, "hero": "O"}],
            "start_health": {"1": 30, "2": 30},
            "rounds": rounds,
        })
    }

    fn with_health(mut r: Value, own: i64) -> Value {
        r["own_health_after"] = json!(own);
        r["health_after"] = json!({"1": own, "2": 30});
        r
    }

    #[test]
    fn damage_taken_prefers_the_games_hit_over_the_health_rule() {
        // Round 1: lost 5 health, the game hit for 4 (1 health was paid in
        // the shop). Round 2: no replay, 3 health lost: inferred.
        let g = game(vec![
            with_health(round(1, Some("lost"), Some(vec![hit("own", 4)])), 25),
            with_health(round(2, Some("lost"), None), 22),
        ]);
        assert_eq!(
            damage_taken(&g),
            [
                Taken {
                    amount: 4,
                    from_log: true
                },
                Taken {
                    amount: 3,
                    from_log: false
                }
            ]
        );
    }

    #[test]
    fn the_last_combat_counts_from_the_games_hit_even_without_health_after_it() {
        let mut last = round(2, Some("lost"), Some(vec![hit("own", 15)]));
        last["own_health_after"] = Value::Null;
        let g = game(vec![
            with_health(round(1, Some("won"), Some(vec![hit("opponent", 6)])), 30),
            last,
        ]);
        let taken: Vec<_> = damage_taken(&g)
            .iter()
            .map(|t| (t.amount, t.from_log))
            .collect();
        assert_eq!(taken, [(0, true), (15, true)]);
    }
}
