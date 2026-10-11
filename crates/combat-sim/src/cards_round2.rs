//! Card coverage, round 2 (T-604, session 071): the cards most seen on the
//! user's boards that round 1 left "not modelled", from their card texts in
//! HearthstoneJSON (D-038). Same rules as [`crate::cards`]: exact card ids,
//! golden cards written out, approximations named with the reason.

use crate::cards::{approx, exact, Action, CardRules, Trigger};
use crate::model::{Keywords, Tribes};

const ANY: Tribes = Tribes::NONE;
const SPELL: &str = "the spell's values can grow during the game; the printed ones are used";
const ATTACK_DAMAGE: &str = "counts damage from attacks only, not from effects";
const LEFT_MOST: &str = "which Deathrattle it triggers and when is not checked against the game yet";
const GEM_CHAIN: &str = "Blood Gems played on it in combat are not passed on";
const KNIGHTS: &str = "Eternal Knights grow with the Knights that died this game, which the log does not show";

/// "Give your minions +3/+2. Repeat for your Dragons. Repeat for your
/// minions with Divine Shield." (Mighty Dragonbreath, cast in combat.)
const DRAGONBREATH: [Action; 3] = [
    Action::BuffAll {
        attack: 3,
        health: 2,
        tribe: ANY,
        others: false,
    },
    Action::BuffAll {
        attack: 3,
        health: 2,
        tribe: Tribes::DRAGON,
        others: false,
    },
    Action::BuffAllWithKeyword {
        attack: 3,
        health: 2,
        keyword: Keywords::DIVINE_SHIELD,
    },
];
const DRAGONBREATH_TWICE: [Action; 6] = [
    DRAGONBREATH[0],
    DRAGONBREATH[1],
    DRAGONBREATH[2],
    DRAGONBREATH[0],
    DRAGONBREATH[1],
    DRAGONBREATH[2],
];

const fn gain(attack: i32, health: i32) -> Action {
    Action::GainStats { attack, health }
}

const fn beasts(attack: i32, health: i32) -> Action {
    Action::BuffAll {
        attack,
        health,
        tribe: Tribes::BEAST,
        others: false,
    }
}

const fn rightmost_undead(times: i32) -> Action {
    Action::GiveRebornAttackToRightmost {
        tribe: Tribes::UNDEAD,
        times,
    }
}

const PER_TRIBE: Action = Action::BuffOnePerTribe {
    attack: 15,
    health: 15,
};

pub const EFFECTS_2: &[(&str, CardRules)] = &[
    // Crimson Vindicator: Rally: cast Mighty Dragonbreath (golden: twice).
    (
        "BG36_241",
        approx(SPELL, &[Trigger::Rally(&DRAGONBREATH)]),
    ),
    (
        "BG36_241_G",
        approx(SPELL, &[Trigger::Rally(&DRAGONBREATH_TWICE)]),
    ),
    // Devout Hellcaller: after another friendly Demon deals damage, gain
    // +2/+2 (golden +4/+4).
    (
        "BG33_155",
        approx(
            ATTACK_DAMAGE,
            &[Trigger::AfterFriendlyDealsDamage {
                tribe: Tribes::DEMON,
                actions: &[gain(2, 2)],
            }],
        ),
    ),
    (
        "BG33_155_G",
        approx(
            ATTACK_DAMAGE,
            &[Trigger::AfterFriendlyDealsDamage {
                tribe: Tribes::DEMON,
                actions: &[gain(4, 4)],
            }],
        ),
    ),
    // Deathstrider: after a friendly Rally minion attacks, trigger the
    // left-most Deathrattle (golden: twice).
    (
        "BG36_208",
        approx(LEFT_MOST, &[Trigger::AfterFriendlyRallyAttacks { times: 1 }]),
    ),
    (
        "BG36_208_G",
        approx(LEFT_MOST, &[Trigger::AfterFriendlyRallyAttacks { times: 2 }]),
    ),
    // Geomagus Roogug: passes Blood Gems on; its Divine Shield is on the board.
    ("BG28_583", approx(GEM_CHAIN, &[])),
    ("BG28_583_G", approx(GEM_CHAIN, &[])),
    // Snazzy Phantom: after a friendly minion is Reborn, give its Attack
    // (golden: double) to the right-most Undead.
    (
        "BG36_515",
        exact(&[Trigger::AfterFriendlyReborn(&[rightmost_undead(1)])]),
    ),
    (
        "BG36_515_G",
        exact(&[Trigger::AfterFriendlyReborn(&[rightmost_undead(2)])]),
    ),
    // Barrier Banshee: after a friendly minion is Reborn, gain Divine Shield
    // and +8/+8 (golden +16/+16).
    (
        "BG36_514",
        exact(&[Trigger::AfterFriendlyReborn(&[
            Action::GainKeyword(Keywords::DIVINE_SHIELD),
            gain(8, 8),
        ])]),
    ),
    (
        "BG36_514_G",
        exact(&[Trigger::AfterFriendlyReborn(&[
            Action::GainKeyword(Keywords::DIVINE_SHIELD),
            gain(16, 16),
        ])]),
    ),
    // Eternal Tycoon: Avenge (5): summon an Eternal Knight (golden: a golden
    // one) that attacks at once.
    (
        "BG34_403",
        approx(
            KNIGHTS,
            &[Trigger::Avenge(
                5,
                &[Action::SummonAndAttack { token: "BG25_008" }],
            )],
        ),
    ),
    (
        "BG34_403_G",
        approx(
            KNIGHTS,
            &[Trigger::Avenge(
                5,
                &[Action::SummonAndAttack {
                    token: "BG25_008_G",
                }],
            )],
        ),
    ),
    // The Last One Standing: Rally: give a friendly minion of each type
    // +15/+15 (golden: twice).
    ("BG34_320", exact(&[Trigger::Rally(&[PER_TRIBE])])),
    ("BG34_320_G", exact(&[Trigger::Rally(&[PER_TRIBE, PER_TRIBE])])),
    // Cage Gnawer: whenever a friendly Beast attacks, give your Beasts
    // +2/+1 (golden +4/+2).
    (
        "BG36_211",
        exact(&[Trigger::WheneverFriendlyAttacks {
            tribe: Tribes::BEAST,
            actions: &[beasts(2, 1)],
        }]),
    ),
    (
        "BG36_211_G",
        exact(&[Trigger::WheneverFriendlyAttacks {
            tribe: Tribes::BEAST,
            actions: &[beasts(4, 2)],
        }]),
    ),
];
