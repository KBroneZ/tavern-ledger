//! Dark Gifts (T-527, D-100): this season's extra effects that ride on a
//! minion. A gifted minion is the card plus its gifts; a gift may add
//! triggers (built from the same primitives as the cards), keywords, every
//! minion type or the golden card's rules.
//!
//! Every gift is keyed by the card id of the gift itself, the spell the
//! offer names (`DARK_GIFT_ENTITY`, D-058), not by the enchantment the game
//! attaches to the minion; `docs/research/dark-gifts.md` maps one to the
//! other. A gift in no list here is not modelled: the odds are blocked, as
//! for a card (D-088). Gifts that act only in the shop, or only add stats
//! the board already shows, are known and change nothing in combat.
//!
//! Written from the gift texts in HearthstoneJSON and the user's own logs;
//! no simulator code was read (clean room, CLAUDE.md rule 1).

use crate::cards::{self, Action, Fidelity, Trigger};
use crate::model::Keywords;

/// What a gift adds to the minion that carries it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Gift {
    pub id: &'static str,
    pub triggers: &'static [Trigger],
    /// Keywords the gift gives (the board shows them too).
    pub keywords: Keywords,
    /// "Has all minion types".
    pub all_types: bool,
    /// "This is Golden": the minion plays as its golden card.
    pub golden: bool,
    pub fidelity: Fidelity,
}

const NONE: Gift = Gift {
    id: "",
    triggers: &[],
    keywords: Keywords::NONE,
    all_types: false,
    golden: false,
    fidelity: Fidelity::Exact,
};

const fn plain(id: &'static str) -> Gift {
    Gift { id, ..NONE }
}

const fn acts(id: &'static str, triggers: &'static [Trigger]) -> Gift {
    Gift {
        id,
        triggers,
        ..NONE
    }
}

const fn keywords(id: &'static str, keywords: Keywords) -> Gift {
    Gift {
        id,
        keywords,
        ..NONE
    }
}

/// The Golemancy gift's Golem, as the user's logs show it.
pub const GOLEM: &str = "BG29_864t";

/// A Rally or Deathrattle whose effect lands outside combat (a card to the
/// hand, free Refreshes): nothing happens in combat, but the minion has a
/// Rally or a Deathrattle for the cards that care.
const OUTSIDE_RALLY: &[Trigger] = &[Trigger::Rally(&[])];
const OUTSIDE_DEATHRATTLE: &[Trigger] = &[Trigger::Deathrattle(&[])];

/// Persisting Horror: back with its Attack, full Health and keywords when it
/// died; whether its gifts and this combat's buffs stay is not checked.
const PERSISTING_HORROR: &str =
    "comes back whole with its keywords; what it keeps is not checked against the logs";

/// Every Dark Gift of season 36 (the card data's 47 gift spells).
pub const GIFTS: &[Gift] = &[
    // Combat effects.
    acts(
        "BG36_MidGameEffect_000t",
        &[Trigger::Deathrattle(&[Action::GiveOwnStats {
            attack: true,
            health: false,
        }])],
    ),
    acts(
        "BG36_MidGameEffect_000t2",
        &[Trigger::Deathrattle(&[Action::GiveOwnStats {
            attack: false,
            health: true,
        }])],
    ),
    acts("BG36_MidGameEffect_000t3", OUTSIDE_RALLY),
    acts("BG36_MidGameEffect_000t5", OUTSIDE_DEATHRATTLE),
    acts(
        "BG36_MidGameEffect_000t7",
        &[Trigger::StartOfCombat(&[Action::MultiplyStats {
            attack: 1,
            health: 2,
        }])],
    ),
    acts(
        "BG36_MidGameEffect_000t9",
        &[Trigger::StartOfCombat(&[Action::GainLeftAttack])],
    ),
    Gift {
        id: "BG36_MidGameEffect_000t12",
        triggers: &[Trigger::RebornWithStats],
        keywords: Keywords::REBORN,
        fidelity: Fidelity::Approximate(PERSISTING_HORROR),
        ..NONE
    },
    keywords(
        "BG36_MidGameEffect_000t13",
        Keywords::DIVINE_SHIELD.union(Keywords::WINDFURY),
    ),
    Gift {
        id: "BG36_MidGameEffect_000t14",
        golden: true,
        ..NONE
    },
    acts("BG36_MidGameEffect_000t15", &[Trigger::ShieldTakesHits(3)]),
    acts(
        "BG36_MidGameEffect_000t16",
        &[Trigger::StartOfCombat(&[Action::TriggerOwnDeathrattles])],
    ),
    Gift {
        id: "BG36_MidGameEffect_000t22",
        all_types: true,
        ..NONE
    },
    acts("BG36_MidGameEffect_000t52", OUTSIDE_DEATHRATTLE),
    acts(
        "BG36_MidGameEffect_000t60",
        &[Trigger::ImmuneWhileAttacking],
    ),
    acts(
        "BG36_MidGameEffect_000t61",
        &[Trigger::Deathrattle(&[Action::SummonWithStats {
            token: GOLEM,
            factor: 1,
            attack_first: false,
        }])],
    ),
    acts("BG36_MidGameEffect_000t66", OUTSIDE_RALLY),
    keywords("BG36_MidGameEffect_000t69", Keywords::VENOMOUS),
    acts(
        "BG36_MidGameEffect_000t71",
        &[Trigger::StartOfCombat(&[Action::MultiplyStats {
            attack: 2,
            health: 1,
        }])],
    ),
    keywords("BG36_MidGameEffect_000t79", Keywords::STEALTH),
    acts("BG36_MidGameEffect_000t80", OUTSIDE_RALLY),
    acts(
        "BG36_MidGameEffect_000t81",
        &[Trigger::StartOfCombat(&[Action::MultiplyStats {
            attack: 3,
            health: 3,
        }])],
    ),
    // "Gain the stats of your Deity": the Deity's stats are not on any
    // board the app has. Harmless on a board taken after start of combat.
    acts(
        "BG36_MidGameEffect_000t89",
        &[Trigger::StartOfCombatUnmodelled],
    ),
    acts(
        "BG36_MidGameEffect_000t90",
        &[Trigger::StartOfCombat(&[Action::SummonCopy])],
    ),
    // Shop only (end of turn, playing cards, copies, Refreshes), or stats
    // the board already shows.
    plain("BG36_MidGameEffect_000t4"),
    plain("BG36_MidGameEffect_000t10"),
    plain("BG36_MidGameEffect_000t11"),
    plain("BG36_MidGameEffect_000t18"),
    plain("BG36_MidGameEffect_000t21"),
    plain("BG36_MidGameEffect_000t28"),
    plain("BG36_MidGameEffect_000t28t"),
    plain("BG36_MidGameEffect_000t29"),
    plain("BG36_MidGameEffect_000t29t"),
    plain("BG36_MidGameEffect_000t30"),
    plain("BG36_MidGameEffect_000t30t"),
    plain("BG36_MidGameEffect_000t50"),
    plain("BG36_MidGameEffect_000t51"),
    plain("BG36_MidGameEffect_000t62"),
    plain("BG36_MidGameEffect_000t64"),
    plain("BG36_MidGameEffect_000t64t"),
    plain("BG36_MidGameEffect_000t65"),
    plain("BG36_MidGameEffect_000t72"),
    plain("BG36_MidGameEffect_000t73"),
    plain("BG36_MidGameEffect_000t74"),
    plain("BG36_MidGameEffect_000t75"),
    plain("BG36_MidGameEffect_000t78"),
    plain("BG36_MidGameEffect_000t82"),
    plain("BG36_MidGameEffect_000t88"),
];

/// The rules for a gift id. `None`: not modelled.
pub fn gift(id: &str) -> Option<&'static Gift> {
    GIFTS.iter().find(|g| g.id == id)
}

/// True when the gift acts at the start of combat (played or not).
pub fn has_start_of_combat(id: &str) -> bool {
    gift(id).is_some_and(|g| {
        g.triggers.iter().any(|t| {
            matches!(
                t,
                Trigger::StartOfCombat(_) | Trigger::StartOfCombatUnmodelled
            )
        })
    })
}

/// The golden card of a card id, as Battlegrounds names most of them.
pub fn golden_id(card_id: &str) -> String {
    format!("{card_id}_G")
}

const GILDED_GAP: &str = "gilded, but its golden card is not modelled: it plays the plain card";

/// How closely the engine plays `gift` on `card_id`: Gilding is exact only
/// when the golden card is modelled or the card has no combat effect.
pub fn fidelity(gift: &Gift, card_id: &str) -> Fidelity {
    if !gift.golden {
        return gift.fidelity;
    }
    let golden_known = cards::rules(&golden_id(card_id)).is_some();
    let no_effect = cards::rules(card_id).is_some_and(|r| r.triggers.is_empty());
    if golden_known || no_effect {
        Fidelity::Exact
    } else {
        Fidelity::Approximate(GILDED_GAP)
    }
}

#[cfg(test)]
#[path = "gifts_tests.rs"]
mod tests;
