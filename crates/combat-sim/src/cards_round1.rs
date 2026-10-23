//! Card coverage, round 1: the first cards the engine plays,
//! from their card texts in HearthstoneJSON (D-038). The rules for writing
//! them are in [`crate::cards`].

use crate::cards::{approx, exact, Action, CardRules, GemTo, Trigger};
use crate::model::{Keywords, Tribes};

const ANY: Tribes = Tribes::NONE;

const fn summon(token: &'static str, count: u8) -> Action {
    Action::Summon { token, count }
}

pub(crate) const GAME_BUFFS: &str =
    "summons tokens that may have game-long buffs that are not known";
pub(crate) const SCALES: &str = "its numbers grow during the game; the base values are used";
const DEITY: &str = "buffs the Deity, which is not on the board";
/// Played with the board's upgrades when it brings them (T-610).
pub(crate) const BLOOD_GEM: &str = "Blood Gems are played as +1/+1; their upgrades are not known";
const RANDOM_PICK: &str = "which minions it picks is not checked against the game yet";

/// Cards with a combat effect the engine plays, by exact card id.
pub const EFFECTS: &[(&str, CardRules)] = &[
    // Deathrattle summons.
    (
        "BG29_611",
        exact(&[Trigger::Deathrattle(&[summon("BG_BOT_312t", 1)])]),
    ),
    (
        "BG29_611_G",
        exact(&[Trigger::Deathrattle(&[summon("TB_BaconUps_032t", 1)])]),
    ),
    (
        "BG28_300",
        exact(&[Trigger::Deathrattle(&[summon("BG_ICC_026t", 2)])]),
    ),
    (
        "BG28_300_G",
        exact(&[Trigger::Deathrattle(&[summon("BG_ICC_026t", 4)])]),
    ),
    (
        "BG30_125",
        exact(&[Trigger::Deathrattle(&[summon("BG_ICC_026t", 3)])]),
    ),
    (
        "BG30_125_G",
        exact(&[Trigger::Deathrattle(&[summon("BG_ICC_026t", 6)])]),
    ),
    (
        "BG25_010",
        exact(&[Trigger::Deathrattle(&[summon("BG25_010t", 1)])]),
    ),
    (
        "BG25_010_G",
        exact(&[Trigger::Deathrattle(&[summon("BG25_010t", 2)])]),
    ),
    (
        "BG31_803",
        approx(
            GAME_BUFFS,
            &[Trigger::Deathrattle(&[summon("BG28_603t", 1)])],
        ),
    ),
    (
        "BG31_803_G",
        approx(
            GAME_BUFFS,
            &[Trigger::Deathrattle(&[summon("BG28_603t", 2)])],
        ),
    ),
    (
        "BG31_801",
        approx(
            GAME_BUFFS,
            &[Trigger::Deathrattle(&[summon("BG28_603t", 1)])],
        ),
    ),
    (
        "BG31_801_G",
        approx(
            GAME_BUFFS,
            &[Trigger::Deathrattle(&[summon("BG28_603t", 2)])],
        ),
    ),
    (
        "BG36_116",
        exact(&[Trigger::Deathrattle(&[
            summon("BGFYM_002t", 1),
            Action::BuffAll {
                attack: 1,
                health: 0,
                tribe: ANY,
                others: false,
            },
        ])]),
    ),
    (
        "BG36_116_G",
        exact(&[Trigger::Deathrattle(&[
            summon("BGFYM_002t", 2),
            Action::BuffAll {
                attack: 2,
                health: 0,
                tribe: ANY,
                others: false,
            },
        ])]),
    ),
    (
        "BG25_009",
        approx(SCALES, &[Trigger::Deathrattle(&[summon("BG25_008", 1)])]),
    ),
    (
        "BG25_009_G",
        approx(SCALES, &[Trigger::Deathrattle(&[summon("BG25_008_G", 1)])]),
    ),
    // Deathrattle buffs.
    (
        "BG36_202",
        approx(
            SCALES,
            &[Trigger::Deathrattle(&[Action::BuffRandom {
                attack: 2,
                health: 1,
                tribe: Tribes::BEAST,
                count: 1,
            }])],
        ),
    ),
    (
        "BG36_202_G",
        approx(
            SCALES,
            &[Trigger::Deathrattle(&[Action::BuffRandom {
                attack: 4,
                health: 2,
                tribe: Tribes::BEAST,
                count: 1,
            }])],
        ),
    ),
    (
        "BG25_022",
        exact(&[Trigger::Deathrattle(&[Action::BuffRandom {
            attack: 1,
            health: 2,
            tribe: Tribes::UNDEAD,
            count: 1,
        }])]),
    ),
    (
        "BG25_022_G",
        exact(&[Trigger::Deathrattle(&[Action::BuffRandom {
            attack: 2,
            health: 4,
            tribe: Tribes::UNDEAD,
            count: 1,
        }])]),
    ),
    (
        "BG33_828",
        exact(&[Trigger::Deathrattle(&[Action::BuffAll {
            attack: 6,
            health: 6,
            tribe: ANY,
            others: false,
        }])]),
    ),
    (
        "BG33_828_G",
        exact(&[Trigger::Deathrattle(&[Action::BuffAll {
            attack: 12,
            health: 12,
            tribe: ANY,
            others: false,
        }])]),
    ),
    (
        "BG28_309",
        approx(
            RANDOM_PICK,
            &[Trigger::Deathrattle(&[Action::GiveKeyword {
                keyword: Keywords::REBORN,
                tribe: Tribes::UNDEAD,
                count: 1,
            }])],
        ),
    ),
    (
        "BG28_309_G",
        approx(
            RANDOM_PICK,
            &[Trigger::Deathrattle(&[Action::GiveKeyword {
                keyword: Keywords::REBORN,
                tribe: Tribes::UNDEAD,
                count: 2,
            }])],
        ),
    ),
    (
        "BGS_040",
        approx(
            RANDOM_PICK,
            &[Trigger::Deathrattle(&[Action::GiveKeyword {
                keyword: Keywords::DIVINE_SHIELD,
                tribe: Tribes::DRAGON,
                count: 3,
            }])],
        ),
    ),
    (
        "TB_BaconUps_154",
        approx(
            RANDOM_PICK,
            &[Trigger::Deathrattle(&[Action::GiveKeyword {
                keyword: Keywords::DIVINE_SHIELD,
                tribe: Tribes::DRAGON,
                count: 6,
            }])],
        ),
    ),
    // The Deity is not a board minion: these deathrattles change nothing on
    // the board, but the Deity's own part in combat is unknown.
    ("BG36_113", approx(DEITY, &[])),
    ("BG36_113_G", approx(DEITY, &[])),
    ("BG36_318", approx(DEITY, &[])),
    ("BG36_318_G", approx(DEITY, &[])),
    ("BGDUO_701", approx(DEITY, &[])),
    ("BGDUO_701_G", approx(DEITY, &[])),
    // Rally.
    (
        "BG29_888",
        exact(&[Trigger::Rally(&[Action::GainStats {
            attack: 2,
            health: 0,
        }])]),
    ),
    (
        "BG29_888_G",
        exact(&[Trigger::Rally(&[Action::GainStats {
            attack: 4,
            health: 0,
        }])]),
    ),
    (
        "BG36_207",
        exact(&[Trigger::Rally(&[Action::BuffAll {
            attack: 4,
            health: 1,
            tribe: ANY,
            others: true,
        }])]),
    ),
    (
        "BG36_207_G",
        exact(&[Trigger::Rally(&[Action::BuffAll {
            attack: 8,
            health: 2,
            tribe: ANY,
            others: true,
        }])]),
    ),
    (
        "BG36_200",
        exact(&[Trigger::Rally(&[summon("BG36_200t", 1)])]),
    ),
    (
        "BG36_200_G",
        exact(&[Trigger::Rally(&[summon("BG36_200t", 2)])]),
    ),
    (
        "BG36_210",
        approx(SCALES, &[Trigger::Rally(&[summon("BG36_202", 1)])]),
    ),
    (
        "BG36_210_G",
        approx(SCALES, &[Trigger::Rally(&[summon("BG36_202_G", 1)])]),
    ),
    (
        "BG36_849",
        exact(&[
            Trigger::Rally(&[Action::GainKeyword(Keywords::DIVINE_SHIELD)]),
            Trigger::AttackImmediately(1),
        ]),
    ),
    (
        "BG36_849_G",
        exact(&[
            Trigger::Rally(&[Action::GainKeyword(Keywords::DIVINE_SHIELD)]),
            Trigger::AttackImmediately(2),
        ]),
    ),
    (
        "BG27_017",
        exact(&[Trigger::Rally(&[Action::DamageTargetAndAdjacent])]),
    ),
    // Golden: both minions next to the target.
    (
        "BG27_017_G",
        exact(&[Trigger::Rally(&[Action::DamageTargetAndNeighbours])]),
    ),
    (
        "BG34_604",
        exact(&[Trigger::Rally(&[Action::GainTargetAttack { times: 1 }])]),
    ),
    (
        "BG34_604_G",
        exact(&[Trigger::Rally(&[Action::GainTargetAttack { times: 2 }])]),
    ),
    (
        "BG33_318",
        approx(
            RANDOM_PICK,
            &[Trigger::Rally(&[Action::GiveKeyword {
                keyword: Keywords::VENOMOUS,
                tribe: Tribes::MURLOC,
                count: 1,
            }])],
        ),
    ),
    (
        "BG33_318_G",
        approx(
            RANDOM_PICK,
            &[Trigger::Rally(&[Action::GiveKeyword {
                keyword: Keywords::VENOMOUS,
                tribe: Tribes::MURLOC,
                count: 2,
            }])],
        ),
    ),
    // Blood Gems (+1/+1 each plus the side's upgrades, T-609).
    (
        "BG33_886",
        approx(
            BLOOD_GEM,
            &[Trigger::Rally(&[Action::Gems {
                to: GemTo::Itself,
                count: 1,
            }])],
        ),
    ),
    (
        "BG33_886_G",
        approx(
            BLOOD_GEM,
            &[Trigger::Rally(&[Action::Gems {
                to: GemTo::Itself,
                count: 2,
            }])],
        ),
    ),
    (
        "BG33_883",
        approx(
            BLOOD_GEM,
            &[Trigger::Rally(&[Action::Gems {
                to: GemTo::Itself,
                count: 4,
            }])],
        ),
    ),
    (
        "BG33_883_G",
        approx(
            BLOOD_GEM,
            &[Trigger::Rally(&[Action::Gems {
                to: GemTo::Itself,
                count: 8,
            }])],
        ),
    ),
    (
        "BG20_104",
        approx(
            BLOOD_GEM,
            &[Trigger::Rally(&[Action::Gems {
                to: GemTo::Others,
                count: 1,
            }])],
        ),
    ),
    (
        "BG20_104_G",
        approx(
            BLOOD_GEM,
            &[Trigger::Rally(&[Action::Gems {
                to: GemTo::Others,
                count: 2,
            }])],
        ),
    ),
    (
        "BG33_430",
        approx(
            BLOOD_GEM,
            &[Trigger::FriendlyAttacks {
                tribe: ANY,
                actions: &[Action::Gems {
                    to: GemTo::Attacker,
                    count: 1,
                }],
            }],
        ),
    ),
    // Other attack triggers.
    (
        "BG29_816",
        exact(&[Trigger::FriendlyAttacks {
            tribe: Tribes::DRAGON,
            actions: &[Action::BuffAttacker {
                attack: 3,
                health: 1,
            }],
        }]),
    ),
    (
        "BG29_816_G",
        exact(&[Trigger::FriendlyAttacks {
            tribe: Tribes::DRAGON,
            actions: &[Action::BuffAttacker {
                attack: 6,
                health: 2,
            }],
        }]),
    ),
    (
        "BG36_102",
        exact(&[Trigger::AfterAttackHitHighestHealth { times: 1 }]),
    ),
    (
        "BG36_102_G",
        exact(&[Trigger::AfterAttackHitHighestHealth { times: 2 }]),
    ),
    (
        "BG25_016",
        exact(&[Trigger::Rally(&[Action::StripTarget(
            Keywords::REBORN.union(Keywords::TAUNT),
        )])]),
    ),
    (
        "BG25_016_G",
        exact(&[Trigger::Rally(&[Action::StripTarget(
            Keywords::REBORN.union(Keywords::TAUNT),
        )])]),
    ),
    (
        "BG23_318",
        exact(&[Trigger::Deathrattle(&[Action::DestroyKiller])]),
    ),
    (
        "BG23_318_G",
        exact(&[Trigger::Deathrattle(&[Action::DestroyKiller])]),
    ),
    (
        "BGS_018",
        exact(&[Trigger::Deathrattle(&[Action::BuffAll {
            attack: 7,
            health: 7,
            tribe: Tribes::BEAST,
            others: false,
        }])]),
    ),
    (
        "TB_BaconUps_085",
        exact(&[Trigger::Deathrattle(&[Action::BuffAll {
            attack: 14,
            health: 14,
            tribe: Tribes::BEAST,
            others: false,
        }])]),
    ),
    (
        "BG31_809",
        approx(
            GAME_BUFFS,
            &[Trigger::Deathrattle(&[summon("BG28_603t", 1)])],
        ),
    ),
    (
        "BG31_809_G",
        approx(
            GAME_BUFFS,
            &[Trigger::Deathrattle(&[summon("BG28_603t", 2)])],
        ),
    ),
    (
        "BG26_963_G",
        exact(&[Trigger::StartOfCombat(&[Action::BuffAll {
            attack: 2,
            health: 2,
            tribe: Tribes::DRAGON,
            others: true,
        }])]),
    ),
    ("BGS_126", exact(&[Trigger::ExcessToAdjacent])),
    ("BG26_817", exact(&[Trigger::Cleave])),
    ("BG26_817_G", exact(&[Trigger::Cleave])),
    // Static and avenge.
    ("BG25_354", exact(&[Trigger::ExtraDeathrattles(1)])),
    ("BG25_354_G", exact(&[Trigger::ExtraDeathrattles(2)])),
    (
        "BG34_405",
        exact(&[
            Trigger::TauntWhileDivineShield,
            Trigger::Avenge(3, &[Action::GainKeyword(Keywords::DIVINE_SHIELD)]),
        ]),
    ),
    (
        "BG34_405_G",
        exact(&[
            Trigger::TauntWhileDivineShield,
            Trigger::Avenge(3, &[Action::GainKeyword(Keywords::DIVINE_SHIELD)]),
        ]),
    ),
    ("BG35_814", exact(&[Trigger::ShieldAtAttack(6)])),
    ("BG35_814_G", exact(&[Trigger::ShieldAtAttack(6)])),
    (
        "BG25_008",
        approx("grows when another Eternal Knight dies; not played", &[]),
    ),
    (
        "BG25_008_G",
        approx("grows when another Eternal Knight dies; not played", &[]),
    ),
    // Start of combat (only played when the board is taken before it).
    (
        "BG36_364",
        exact(&[Trigger::StartOfCombat(&[hopebringer(4, 3)])]),
    ),
    (
        "BG36_364_G",
        exact(&[Trigger::StartOfCombat(&[hopebringer(8, 6)])]),
    ),
    (
        "BG26_963",
        exact(&[Trigger::StartOfCombat(&[Action::BuffAll {
            attack: 1,
            health: 1,
            tribe: Tribes::DRAGON,
            others: true,
        }])]),
    ),
    // Amber Guardian: another friendly Dragon +2/+2 and Divine Shield
    // (golden: two), T-628.
    ("BG24_500", exact(&[Trigger::StartOfCombat(&[amber(1)])])),
    // Runic Arcanist: casts Shiny Ring ("give your minions +1/+1") twice
    // (golden: four times), with the side's spell upgrades (T-628). Its gap
    // matters only when its start of combat is played: a board taken after
    // it already has the casts.
    (
        "BG36_245",
        approx(
            crate::cards_round2::SPELL,
            &[Trigger::StartOfCombat(&[SHINY_RING, SHINY_RING])],
        ),
    ),
    (
        "BG36_245_G",
        approx(
            crate::cards_round2::SPELL,
            &[Trigger::StartOfCombat(&[
                SHINY_RING, SHINY_RING, SHINY_RING, SHINY_RING,
            ])],
        ),
    ),
    ("BG26_805", exact(&[Trigger::StartOfCombatUnmodelled])),
    ("BG26_354", exact(&[Trigger::StartOfCombatUnmodelled])),
    ("BG27_556", exact(&[Trigger::StartOfCombatUnmodelled])),
    ("BG34_142", exact(&[Trigger::StartOfCombatUnmodelled])),
    // Ultraviolet Ascendant: your other Elementals gain its numbers (T-628).
    (
        "BG31_810",
        exact(&[Trigger::StartOfCombat(&[ultraviolet(3)])]),
    ),
    // Boom-in-a-Box: 3 damage to every other minion (golden: twice), T-628.
    (
        "BG36_620",
        exact(&[Trigger::StartOfCombat(&[Action::DamageAllOthers(3)])]),
    ),
    // Golden forms of the cards above.
    ("BG24_500_G", exact(&[Trigger::StartOfCombat(&[amber(2)])])),
    ("BG26_805_G", exact(&[Trigger::StartOfCombatUnmodelled])),
    ("BG34_142_G", exact(&[Trigger::StartOfCombatUnmodelled])),
    (
        "BG31_810_G",
        exact(&[Trigger::StartOfCombat(&[ultraviolet(6)])]),
    ),
    (
        "BG36_620_G",
        exact(&[Trigger::StartOfCombat(&[
            Action::DamageAllOthers(3),
            Action::DamageAllOthers(3),
        ])]),
    ),
];

/// Hopebringer: your minions gain its numbers, which grow during the game
/// (the user's logs: its TAG_SCRIPT_DATA_NUM_1 and _2 are the buff it gives);
/// a board taken after start of combat already has the buff (T-628).
const fn hopebringer(attack: i32, health: i32) -> Action {
    Action::BuffAllByCounters {
        attack,
        health,
        tribe: ANY,
        others: false,
    }
}

/// Ultraviolet Ascendant: the other Elementals gain its numbers.
const fn ultraviolet(n: i32) -> Action {
    Action::BuffAllByCounters {
        attack: n,
        health: n,
        tribe: Tribes::ELEMENTAL,
        others: true,
    }
}

/// Amber Guardian's start of combat on `count` other Dragons.
const fn amber(count: u8) -> Action {
    Action::BuffRandomWithKeyword {
        attack: 2,
        health: 2,
        keyword: Keywords::DIVINE_SHIELD,
        tribe: Tribes::DRAGON,
        count,
    }
}

/// Shiny Ring, the Tavern spell Runic Arcanist casts.
const SHINY_RING: Action = Action::Spell(&[Action::BuffAll {
    attack: 1,
    health: 1,
    tribe: ANY,
    others: false,
}]);
