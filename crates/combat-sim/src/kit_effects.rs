//! What the engine plays of the hero powers and trinkets it knows (T-628
//! rounds 2 and 3): their start-of-combat effects and the hero powers that
//! act once "when you have space", from their texts in HearthstoneJSON
//! (D-038) and the user's logs. Names only, as comments. Every other power
//! or trinket that acts in combat is still "not played" ([`crate::kit`]).

use crate::cards::Fidelity;
use crate::kit::{
    COPY_OF_DAMAGED, FIRED_AT_RIGHT_END, HERALD_ORDER, HIGHEST_TIER_TIES, LIGHTNING_REPEATS,
    LOWEST_TIES, PIRATES_WITH_SHIELD, TIMEPIECE_ROUNDING, WINGMEN_ORDER,
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
    /// Destroy the friendly minion the card names
    /// ([`crate::model::KitCard::target`]) and keep an exact copy of it,
    /// summoned back at its place once the side has space.
    DestroyTarget,
    /// Transform the friendly minion the card names into an exact copy of
    /// the teammate's highest-Tier minion (the left-most on ties).
    TransformTargetIntoTeammates,
}

/// Something a hero power does once per combat, the first time its side has
/// space (fewer than seven minions): at its turn in the start of combat,
/// else once the deaths that made the space are resolved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpaceAction {
    /// Summon an exact copy of the friendly minion with the highest Attack
    /// (or Health), the left-most on ties, to its right. Only from round
    /// `unlock` on, and while the card's first number is not 0 (the game
    /// sets it to 0 once it has acted this combat).
    CopyHighest { health: bool, unlock: u32 },
    /// Summon the minion the card holds ([`crate::model::KitCard::held`])
    /// at the right end; it attacks at once.
    FireHeld,
}

/// A played power or trinket: its start-of-combat actions, in order, and
/// what it does when its side has space.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KitPlay {
    pub start: &'static [KitAction],
    pub space: Option<SpaceAction>,
    pub fidelity: Fidelity,
}

const fn exact(start: &'static [KitAction]) -> KitPlay {
    KitPlay {
        start,
        space: None,
        fidelity: Fidelity::Exact,
    }
}

const fn approx(why: &'static str, start: &'static [KitAction]) -> KitPlay {
    KitPlay {
        start,
        space: None,
        fidelity: Fidelity::Approximate(why),
    }
}

const fn on_space(why: &'static str, space: SpaceAction) -> KitPlay {
    KitPlay {
        start: &[],
        space: Some(space),
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
    // Frostwolf Fervor and Stormpike Strength (the user's logs, 10 copies:
    // the highest Attack or Health, the left-most of two equal, summoned to
    // its right with its stats, keywords and buffs; at the start of combat
    // when there is space, else once a death makes some, once per combat;
    // never before round 7 although the first number is 1 then too).
    (
        "BG22_HERO_002p",
        on_space(
            COPY_OF_DAMAGED,
            SpaceAction::CopyHighest {
                health: false,
                unlock: 7,
            },
        ),
    ),
    (
        "BG22_HERO_003p",
        on_space(
            COPY_OF_DAMAGED,
            SpaceAction::CopyHighest {
                health: true,
                unlock: 7,
            },
        ),
    ),
    // Lock and Load (the user's logs: the held minion comes in with its
    // stats and buffs and attacks at once, Taunt first; at the start of
    // combat when there is space, else later).
    (
        "BG22_HERO_000p_Alt",
        on_space(FIRED_AT_RIGHT_END, SpaceAction::FireHeld),
    ),
    // Rapid Reanimation (the user's logs: the marked minion dies, its
    // Deathrattles and Reborn resolve, and its exact copy, damage included,
    // comes back at its old place once there is space).
    ("BG25_HERO_103p", exact(&[KitAction::DestroyTarget])),
    // Glorious Gloop (the user's logs: the marked minion becomes the
    // teammate's highest-Tier minion with its buffs).
    (
        "BGDUO_HERO_101p",
        approx(
            HIGHEST_TIER_TIES,
            &[KitAction::TransformTargetIntoTeammates],
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

/// Every hero power and trinket the engine plays, by card id (the website's
/// table of what each needs is checked against them).
pub fn played_ids() -> impl Iterator<Item = &'static str> {
    PLAYED.iter().map(|(id, _)| *id)
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

/// What a played power needs beside the board, to be played.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Needs {
    /// The friendly minion it names.
    pub target: bool,
    /// The minion it holds.
    pub held: bool,
    /// The game's round.
    pub round: bool,
    /// The teammate's board.
    pub teammate: bool,
}

/// What `play` needs beside the board.
pub fn needs(play: &KitPlay) -> Needs {
    let mut n = Needs::default();
    for action in play.start {
        match action {
            KitAction::DestroyTarget => n.target = true,
            KitAction::TransformTargetIntoTeammates => {
                n.target = true;
                n.teammate = true;
            }
            _ => {}
        }
    }
    match play.space {
        Some(SpaceAction::CopyHighest { .. }) => n.round = true,
        Some(SpaceAction::FireHeld) => n.held = true,
        None => {}
    }
    n
}

/// True when a played power acts this combat: a "copy the highest" power
/// only from its round on and while its first number is not 0, a power that
/// fires a held minion while its first number is not 0. `None` when it
/// depends on the round and the round is not given.
pub fn acts(play: &KitPlay, counters: &[i32; 5], round: Option<u32>) -> Option<bool> {
    match play.space {
        Some(SpaceAction::CopyHighest { unlock, .. }) => {
            round.map(|r| r >= unlock && counters[0] != 0)
        }
        // Lock and Load's first number is 1 while it holds a minion not
        // fired yet (the user's logs, 6 of 6 combats), 0 otherwise.
        Some(SpaceAction::FireHeld) => Some(counters[0] != 0),
        None => Some(true),
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
