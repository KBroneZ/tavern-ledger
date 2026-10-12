//! Card coverage, round 3 (T-608): the cards most seen on the
//! user's Duos boards that rounds 1 and 2 left "not modelled" and that the
//! board alone can play, from their card texts in HearthstoneJSON (D-038).
//! Same rules as [`crate::cards`]: exact card ids, golden cards written out,
//! approximations named with the reason.
//!
//! Left out, most seen first: cards that summon from the hand (`BG34_140`,
//! `BG31_835`, `BG21_HERO_030t`: the hand is not seen), `BG31_999` (copies
//! the minion it destroyed at the start of combat, gone from a logged
//! board), `BG36_360t9` (its Golem token's card id is not in the card data),
//! `TB_BaconShop_HP_105t` (gains other minions' Deathrattles).

use crate::cards::{approx, exact, Action, CardRules, Trigger};
use crate::model::Tribes;

const LATER_UNDEAD: &str = "Undead summoned after it died do not get the Attack";
const IMPROVES: &str = "its bonus grows during the game; the printed one is used";
const SUMMON_ORDER: &str = "its order with other summon effects is not checked yet";

const fn gain(attack: i32, health: i32) -> Action {
    Action::GainStats { attack, health }
}

const fn undead_attack(attack: i32) -> Action {
    Action::BuffAll {
        attack,
        health: 0,
        tribe: Tribes::UNDEAD,
        others: false,
    }
}

const fn on_beast_summon(actions: &'static [Action]) -> Trigger {
    Trigger::AfterFriendlySummon {
        tribe: Tribes::BEAST,
        actions,
    }
}

pub const EFFECTS_3: &[(&str, CardRules)] = &[
    // Falling Sky Golem: +4/+2 (golden +8/+4) for each Deathrattle you
    // trigger; its Divine Shield is on the board.
    (
        "BG35_342",
        exact(&[Trigger::FriendlyDeathrattle(&[gain(4, 2)])]),
    ),
    (
        "BG35_342_G",
        exact(&[Trigger::FriendlyDeathrattle(&[gain(8, 4)])]),
    ),
    // Plaguerunner: Deathrattle: your Undead have +2 Attack (golden +4).
    (
        "BG34_690",
        approx(LATER_UNDEAD, &[Trigger::Deathrattle(&[undead_attack(2)])]),
    ),
    (
        "BG34_690_G",
        approx(LATER_UNDEAD, &[Trigger::Deathrattle(&[undead_attack(4)])]),
    ),
    // Banana Slamma: after you summon a Beast in combat, double its Attack
    // (golden: triple).
    (
        "BG26_802",
        approx(
            SUMMON_ORDER,
            &[on_beast_summon(&[Action::MultiplyAttackerAttack {
                factor: 2,
            }])],
        ),
    ),
    (
        "BG26_802_G",
        approx(
            SUMMON_ORDER,
            &[on_beast_summon(&[Action::MultiplyAttackerAttack {
                factor: 3,
            }])],
        ),
    ),
    // Lurking Leviathan: whenever you summon a Beast, give it +3 Attack
    // (golden +6) and improve this permanently.
    (
        "BG35_602",
        approx(
            IMPROVES,
            &[on_beast_summon(&[Action::BuffAttacker {
                attack: 3,
                health: 0,
            }])],
        ),
    ),
    (
        "BG35_602_G",
        approx(
            IMPROVES,
            &[on_beast_summon(&[Action::BuffAttacker {
                attack: 6,
                health: 0,
            }])],
        ),
    ),
];
