//! The card facts the combat simulator needs that a board does not show
//! (T-605): a Battlegrounds minion's own Attack, Health and lasting keywords
//! (for Reborn), its tavern tier (for damage) and its types. From
//! HearthstoneJSON (D-038), checked like the rest of the card data.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// More than this is not the real data (about 2,500 in 2026).
pub const MAX_COMBAT_CARDS: usize = 10_000;
/// Far above any minion's printed stats.
pub const MAX_STAT: i64 = 1_000;
const MAX_TIER: i64 = 7;
const MAX_TRIBES: usize = 4;
const MAX_NAME: usize = 32;
/// The keywords the simulator tracks, as HearthstoneJSON spells them.
pub const COMBAT_KEYWORDS: [&str; 8] = [
    "TAUNT",
    "DIVINE_SHIELD",
    "REBORN",
    "VENOMOUS",
    "POISONOUS",
    "WINDFURY",
    "MEGA_WINDFURY",
    "STEALTH",
];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CombatFacts {
    pub attack: i32,
    pub health: i32,
    /// Tavern tier; None when the data has none (damage is then unknown).
    pub tier: Option<u8>,
    /// Types as the log names them (`BEAST`, `ALL`); empty for none.
    pub tribes: Vec<String>,
    /// Keywords of [`COMBAT_KEYWORDS`] the card has printed.
    pub keywords: Vec<String>,
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= MAX_NAME
        && name.bytes().all(|b| b.is_ascii_uppercase() || b == b'_')
}

impl CombatFacts {
    /// The facts of one card, or None when it is not a Battlegrounds minion
    /// or a field does not pass the checks (it then is "not known": the
    /// simulator never guesses).
    pub fn from_card(
        set: Option<&str>,
        card_type: Option<&str>,
        attack: Option<i64>,
        health: Option<i64>,
        tier: Option<i64>,
        tribes: Option<Vec<String>>,
        mechanics: Option<&Value>,
    ) -> Option<CombatFacts> {
        if set != Some("BATTLEGROUNDS") || card_type != Some("MINION") {
            return None;
        }
        let stat = |v: Option<i64>| v.filter(|v| (0..=MAX_STAT).contains(v)).map(|v| v as i32);
        let tier = match tier {
            None => None,
            Some(t) if (1..=MAX_TIER).contains(&t) => Some(t as u8),
            Some(_) => return None,
        };
        let keywords = match mechanics {
            None => Vec::new(),
            Some(m) => m
                .as_array()?
                .iter()
                .map(|v| v.as_str())
                .collect::<Option<Vec<&str>>>()?
                .into_iter()
                .filter(|k| COMBAT_KEYWORDS.contains(k))
                .map(String::from)
                .collect(),
        };
        CombatFacts {
            attack: stat(attack)?,
            health: stat(health).filter(|h| *h > 0)?,
            tier,
            tribes: tribes?,
            keywords,
        }
        .clean()
    }

    /// Checks an entry (also when read back from the PC).
    pub fn clean(self) -> Option<CombatFacts> {
        let ok = (0..=MAX_STAT as i32).contains(&self.attack)
            && (1..=MAX_STAT as i32).contains(&self.health)
            && self.tier.is_none_or(|t| (1..=MAX_TIER as u8).contains(&t))
            && self.tribes.len() <= MAX_TRIBES
            && self.tribes.iter().all(|t| valid_name(t))
            && self.keywords.len() <= COMBAT_KEYWORDS.len()
            && self.keywords.iter().all(|k| COMBAT_KEYWORDS.contains(&k.as_str()));
        ok.then_some(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn facts(set: &str, mechanics: Value) -> Option<CombatFacts> {
        CombatFacts::from_card(
            Some(set),
            Some("MINION"),
            Some(2),
            Some(3),
            Some(1),
            Some(vec!["BEAST".into()]),
            Some(&mechanics),
        )
    }

    #[test]
    fn keeps_a_battlegrounds_minions_stats_tier_types_and_keywords() {
        let f = facts("BATTLEGROUNDS", json!(["DIVINE_SHIELD", "BATTLECRY", "POISONOUS"]))
            .expect("a minion");
        assert_eq!((f.attack, f.health, f.tier), (2, 3, Some(1)));
        assert_eq!(f.tribes, ["BEAST"]);
        assert_eq!(f.keywords, ["DIVINE_SHIELD", "POISONOUS"]);
    }

    #[test]
    fn refuses_what_is_not_a_battlegrounds_minion_or_looks_wrong() {
        assert!(facts("CORE", json!([])).is_none());
        assert!(facts("BATTLEGROUNDS", json!("TAUNT")).is_none());
        assert!(facts("BATTLEGROUNDS", json!([1])).is_none());
        let f = |a, h, t| {
            CombatFacts::from_card(
                Some("BATTLEGROUNDS"),
                Some("MINION"),
                a,
                h,
                t,
                Some(vec![]),
                None,
            )
        };
        assert!(f(Some(-1), Some(1), None).is_none());
        assert!(f(Some(1), Some(0), None).is_none());
        assert!(f(None, Some(1), None).is_none());
        assert!(f(Some(1), Some(1), Some(9)).is_none());
        assert_eq!(f(Some(1), Some(1), None).map(|c| c.tier), Some(None));
        let odd_tribe = CombatFacts {
            tribes: vec!["beast".into()],
            ..f(Some(1), Some(1), None).unwrap()
        };
        assert!(odd_tribe.clean().is_none());
    }
}
