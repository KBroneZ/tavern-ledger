//! What the engine plays of the hero powers and trinkets that act during
//! combat (T-628 round 4), from their texts in HearthstoneJSON (D-038) and
//! the user's logs. Names only, as comments. Every other one that acts
//! during combat is still approximate ([`crate::kit`]).

use crate::cards::Fidelity;
use crate::cards_round1::BLOOD_GEM;
use crate::kit::{BEETLES_PLACE, KNIGHTS_SUMMONED, SCALE_ON_SHIELDED, SPROUT_REBORN, STAFF_PICK};
use crate::model::{Keywords, Tribes};

/// Something a hero power or trinket does during combat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KitTrigger {
    /// After a friendly minion kills an enemy, it gains `attack`.
    KillGainsAttack(i32),
    /// After every `every` friendly attacks this combat, play a Blood Gem on
    /// every friendly Quilboar.
    GemsEveryAttacks(u32),
    /// The side's first Deathrattle this combat triggers an extra time.
    FirstDeathrattleTwice,
    /// Minions the side summons in combat gain stats and keywords.
    SummonedGain {
        attack: i32,
        health: i32,
        keywords: Keywords,
    },
    /// Whenever a friendly minion of `tribe` attacks, before the damage, it
    /// gains stats and keywords; at most `limit` times per combat (0: no
    /// limit).
    TribeAttacks {
        tribe: Tribes,
        attack: i32,
        health: i32,
        keywords: Keywords,
        limit: u32,
    },
    /// Avenge (`n`): give a random friendly minion of `tribe` that lacks
    /// `keyword` that keyword.
    AvengeKeyword {
        n: u32,
        tribe: Tribes,
        keyword: Keywords,
    },
    /// After a friendly Undead dies, the side's Eternal Knights have
    /// `attack`/`health` more this game: those on the board gain it, and
    /// those summoned later in the combat come with it.
    UndeadDeathBuffsKnights { attack: i32, health: i32 },
    /// Avenge (`n`): summon `count` tokens at the right end, with
    /// `keywords`.
    AvengeSummon {
        n: u32,
        token: &'static str,
        count: u8,
        keywords: Keywords,
    },
}

/// A played power or trinket that acts during combat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KitPassive {
    pub trigger: KitTrigger,
    pub fidelity: Fidelity,
    /// Played only from a board taken before start of combat: a start of
    /// combat may have used it up before a board taken at the first attack.
    pub before_start_only: bool,
}

const fn exact(trigger: KitTrigger) -> KitPassive {
    KitPassive {
        trigger,
        fidelity: Fidelity::Exact,
        before_start_only: false,
    }
}

const fn approx(why: &'static str, trigger: KitTrigger) -> KitPassive {
    KitPassive {
        trigger,
        fidelity: Fidelity::Approximate(why),
        before_start_only: false,
    }
}

/// The played ones, by card id.
pub const PASSIVE: &[(&str, KitPassive)] = &[
    // Hero powers.
    // Glory of Combat (the user's logs, 25 kills: the friendly minion that
    // killed, attacker or attacked, +1 Attack each kill).
    ("BG20_HERO_100p", exact(KitTrigger::KillGainsAttack(1))),
    // Sprout It Out!
    (
        "TB_BaconShop_HP_107",
        approx(
            SPROUT_REBORN,
            KitTrigger::SummonedGain {
                attack: 1,
                health: 2,
                keywords: Keywords::TAUNT,
            },
        ),
    ),
    // Trinkets.
    // Jar o' Gems (the user's logs, 37 combats' counts: its number goes 2,
    // 1, then the gems and back to 2, every second friendly attack); like
    // the Blood Gem cards, approximate while the side's gem upgrades are not
    // given.
    (
        "BG30_MagicItem_546",
        approx(BLOOD_GEM, KitTrigger::GemsEveryAttacks(2)),
    ),
    // Deathly Phylactery (the user's logs: once per combat).
    (
        "BG30_MagicItem_700",
        KitPassive {
            trigger: KitTrigger::FirstDeathrattleTwice,
            fidelity: Fidelity::Exact,
            before_start_only: true,
        },
    ),
    // Lionfish Portrait.
    (
        "BG36_MagicItem_201",
        exact(KitTrigger::TribeAttacks {
            tribe: Tribes::BEAST,
            attack: 4,
            health: 2,
            keywords: Keywords::NONE,
            limit: 0,
        }),
    ),
    // Faerie Dragon Scale.
    (
        "BG32_MagicItem_363",
        approx(
            SCALE_ON_SHIELDED,
            KitTrigger::TribeAttacks {
                tribe: Tribes::DRAGON,
                attack: 0,
                health: 0,
                keywords: Keywords::DIVINE_SHIELD,
                limit: 3,
            },
        ),
    ),
    // Staff of the Scourge.
    (
        "BG30_MagicItem_437",
        approx(
            STAFF_PICK,
            KitTrigger::AvengeKeyword {
                n: 5,
                tribe: Tribes::UNDEAD,
                keyword: Keywords::REBORN,
            },
        ),
    ),
    // Eternal Portrait (the user's logs, 32 blocks: after each friendly
    // Undead died, every friendly Eternal Knight, golden or not, gained
    // exactly +4/+2).
    (
        "BG36_MagicItem_216",
        approx(
            KNIGHTS_SUMMONED,
            KitTrigger::UndeadDeathBuffsKnights {
                attack: 4,
                health: 2,
            },
        ),
    ),
    // Beetle Band.
    (
        "BG32_MagicItem_860t",
        approx(
            BEETLES_PLACE,
            KitTrigger::AvengeSummon {
                n: 7,
                token: "BG28_603t",
                count: 2,
                keywords: Keywords::TAUNT,
            },
        ),
    ),
];

/// The play of `card_id` during combat, when the engine plays it.
pub fn passive(card_id: &str) -> Option<&'static KitPassive> {
    PASSIVE
        .iter()
        .find(|(id, _)| *id == card_id)
        .map(|(_, p)| p)
}

/// The tokens the played powers and trinkets summon.
pub fn tokens() -> impl Iterator<Item = &'static str> {
    PASSIVE.iter().filter_map(|(_, p)| match p.trigger {
        KitTrigger::AvengeSummon { token, .. } => Some(token),
        _ => None,
    })
}
