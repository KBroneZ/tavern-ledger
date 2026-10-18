//! What the engine plays of the hero powers and trinkets it knows (T-628
//! round 2): their start-of-combat effects, from their texts in
//! HearthstoneJSON (D-038) and the user's logs. Names only, as comments.
//! Every other power or trinket that acts in combat is still "not played"
//! ([`crate::kit`]).

use crate::cards::Fidelity;
use crate::kit::{
    HERALD_ORDER, LIGHTNING_REPEATS, LOWEST_TIES, PIRATES_WITH_SHIELD, TIMEPIECE_ROUNDING,
    WINGMEN_ORDER,
};
use crate::model::{Keywords, Tribes};

/// A number an effect uses: printed, or one of the card's own numbers (the
/// ones its text shows, [`crate::model::KitCard::counters`]), with the
/// printed one when the log gives none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Amount {
    Fixed(i32),
    Counter { index: usize, printed: i32 },
}

/// Which friendly minions an effect picks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pick {
    LeftMost,
    RightMost,
    /// Both ends (one minion when there is one).
    LeftAndRightMost,
    /// `count` different random friendly minions of `tribe`.
    Random {
        tribe: Tribes,
        count: u8,
    },
    /// Every friendly minion that is one of these cards.
    Cards(&'static [&'static str]),
    All,
    /// The `count` friendly minions with the lowest Attack, the left-most
    /// first on equal Attack.
    LowestAttack(u8),
}

/// Something a hero power or trinket does at the start of combat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KitAction {
    Buff {
        to: Pick,
        attack: Amount,
        health: Amount,
    },
    /// One random friendly minion of each minion type gains stats.
    BuffOnePerTribe(Amount),
    Keywords {
        to: Pick,
        keywords: Keywords,
    },
    DoubleAttack(Pick),
    DoubleStats(Pick),
    /// The minions gain Health equal to half their Attack, rounded down.
    HealthFromHalfAttack(Pick),
    /// `count` times, deal `amount` damage to a random enemy minion.
    DamageRandomEnemies {
        amount: i32,
        count: u8,
    },
    /// Trigger every friendly minion's Deathrattles, left to right.
    TriggerDeathrattles,
    /// The minions attack at once, not as one of the side's turns.
    AttackImmediately(Pick),
}

/// A played power or trinket: its start-of-combat actions, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KitPlay {
    pub start: &'static [KitAction],
    pub fidelity: Fidelity,
}

const fn exact(start: &'static [KitAction]) -> KitPlay {
    KitPlay {
        start,
        fidelity: Fidelity::Exact,
    }
}

const fn approx(why: &'static str, start: &'static [KitAction]) -> KitPlay {
    KitPlay {
        start,
        fidelity: Fidelity::Approximate(why),
    }
}

const ENDS: Pick = Pick::LeftAndRightMost;
const SHIELD: Keywords = Keywords::DIVINE_SHIELD;

/// Faceless Converter and its golden card.
const CONVERTERS: &[&str] = &["BG36_318", "BG36_318_G"];

/// The played ones, by card id.
pub const PLAYED: &[(&str, KitPlay)] = &[
    // Hero powers.
    // Wax Warband: its buff is its third number (the user's logs: the gain of
    // each minion equals TAG_SCRIPT_DATA_NUM_3 in 9 of 9 combats; the first
    // is the Gold left to its next improvement).
    (
        "TB_BaconShop_HP_037a",
        exact(&[KitAction::BuffOnePerTribe(Amount::Counter {
            index: 2,
            printed: 1,
        })]),
    ),
    // Swatting Insects.
    (
        "TB_BaconShop_HP_086",
        exact(&[KitAction::Keywords {
            to: Pick::LeftMost,
            keywords: Keywords::WINDFURY
                .union(Keywords::DIVINE_SHIELD)
                .union(Keywords::TAUNT),
        }]),
    ),
    // Wingmen: +2/+1 to both ends, then they attack.
    (
        "TB_BaconShop_HP_069",
        approx(
            WINGMEN_ORDER,
            &[
                KitAction::Buff {
                    to: ENDS,
                    attack: Amount::Fixed(2),
                    health: Amount::Fixed(1),
                },
                KitAction::AttackImmediately(ENDS),
            ],
        ),
    ),
    // Fire Invocation.
    (
        "BG22_HERO_001p_t2",
        exact(&[KitAction::DoubleAttack(Pick::LeftMost)]),
    ),
    // Water Invocation (the user's logs: the right-most minion).
    (
        "BG22_HERO_001p_t3",
        exact(&[KitAction::Keywords {
            to: Pick::RightMost,
            keywords: Keywords::DIVINE_SHIELD.union(Keywords::TAUNT),
        }]),
    ),
    // Lightning Invocation.
    (
        "BG22_HERO_001p_t4",
        approx(
            LIGHTNING_REPEATS,
            &[KitAction::DamageRandomEnemies {
                amount: 1,
                count: 5,
            }],
        ),
    ),
    // Trinkets.
    // Holy Mallet (the user's logs: the two ends as start of combat begins).
    (
        "BG30_MagicItem_902",
        exact(&[KitAction::Keywords {
            to: ENDS,
            keywords: SHIELD,
        }]),
    ),
    // Bronze Timepiece.
    (
        "BG30_MagicItem_995",
        approx(
            TIMEPIECE_ROUNDING,
            &[KitAction::HealthFromHalfAttack(Pick::All)],
        ),
    ),
    // Protective Ring.
    (
        "BG35_MagicItem_711",
        approx(
            PIRATES_WITH_SHIELD,
            &[KitAction::Keywords {
                to: Pick::Random {
                    tribe: Tribes::PIRATE,
                    count: 4,
                },
                keywords: SHIELD,
            }],
        ),
    ),
    // Training Certificate (the user's logs: the two lowest-Attack minions
    // doubled, Attack and Health).
    (
        "BG30_MagicItem_962",
        approx(
            LOWEST_TIES,
            &[KitAction::DoubleStats(Pick::LowestAttack(2))],
        ),
    ),
    // Converter Portrait.
    (
        "BG36_MagicItem_600",
        exact(&[KitAction::Keywords {
            to: Pick::Cards(CONVERTERS),
            keywords: Keywords::REBORN,
        }]),
    ),
    // Herald Sticker.
    (
        "BG32_MagicItem_306",
        approx(HERALD_ORDER, &[KitAction::TriggerDeathrattles]),
    ),
];

/// The play of `card_id`, when the engine plays it.
pub fn played(card_id: &str) -> Option<&'static KitPlay> {
    PLAYED.iter().find(|(id, _)| *id == card_id).map(|(_, p)| p)
}

/// The value of `amount` for a card with these numbers: the printed one when
/// the log gave none (0).
pub fn value(amount: Amount, counters: &[i32; 5]) -> i32 {
    match amount {
        Amount::Fixed(n) => n,
        Amount::Counter { index, printed } => match counters.get(index) {
            Some(&n) if n != 0 => n,
            _ => printed,
        },
    }
}

/// True when an action uses one of the card's numbers that is 0 (not given).
pub fn counter_missing(play: &KitPlay, counters: &[i32; 5]) -> bool {
    let missing = |a: &Amount| match *a {
        Amount::Fixed(_) => false,
        Amount::Counter { index, .. } => counters.get(index).is_none_or(|n| *n == 0),
    };
    play.start.iter().any(|action| match action {
        KitAction::Buff { attack, health, .. } => missing(attack) || missing(health),
        KitAction::BuffOnePerTribe(a) => missing(a),
        _ => false,
    })
}
