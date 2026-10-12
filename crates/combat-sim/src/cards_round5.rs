//! Card coverage, round 5 (T-604): no game was played since
//! round 4, so this round takes the rest of the not-modelled list that the
//! board can play, from the card texts in HearthstoneJSON (D-038), and the
//! hand summoners, which play as approximate since this round (D-097). Same
//! rules as [`crate::cards`]: exact card ids, golden cards written out,
//! approximations named with the reason.
//!
//! Left blocking, and why: `BG31_999` Stitched Salvager (its Deathrattle
//! copies the minion it destroyed at the start of combat, gone from every
//! board the app has; the blocked combats it is in, played anyway, are
//! scored far worse than the rest, section 7.6 of the design doc);
//! `BGS_008` Ghastcoiler and `BG34_Giant_590` Timewarped Festergut (random
//! minions from a pool the engine does not have).

use crate::cards::{approx, exact, Action, CardRules, Trigger};

/// The cards that summon a minion from the hand, which is not seen
/// (section 10, question 4). They play as approximate, the summon left out
/// (D-097); `BG34_Giant_017` summons from the hand on its Deathrattle.
pub const HAND_SUMMONERS: [&str; 8] = [
    "BG34_140",
    "BG34_140_G",
    "BG31_835",
    "BG31_835_G",
    "BG21_HERO_030t",
    "BG21_HERO_030t_G",
    "BG34_Giant_017",
    "BG34_Giant_017_G",
];

const FROM_HAND: &str = "summons a minion from the hand, which is not seen: the summon is left out";
const COPIED_EARLIER: &str =
    "on a board taken after the start of combat, the Deathrattles it copied are not on the board";
const EARLIER_GAINS: &str = "Deathrattles it gained in earlier combats are not on the board";

const fn copy_leftmost(times: u8) -> Action {
    Action::CopyLeftmostDeathrattles { count: 2, times }
}

const HAND: CardRules = approx(FROM_HAND, &[]);

pub const EFFECTS_5: &[(&str, CardRules)] = &[
    // Expert Aviator (Rally), Deathly Striker (Avenge: get an Undead;
    // Deathrattle: summon it), Sneed's New Shredder and Timewarped Scourfin
    // (Deathrattle): all their combat effects summon from the hand. The
    // Scourfin's Taunt is on the board.
    (HAND_SUMMONERS[0], HAND),
    (HAND_SUMMONERS[1], HAND),
    (HAND_SUMMONERS[2], HAND),
    (HAND_SUMMONERS[3], HAND),
    (HAND_SUMMONERS[4], HAND),
    (HAND_SUMMONERS[5], HAND),
    (HAND_SUMMONERS[6], HAND),
    (HAND_SUMMONERS[7], HAND),
    // Foul Egg: Deathrattle: summon a 3/3 Undead Chicken (no golden card).
    (
        "BG26_RLK_833",
        exact(&[Trigger::Deathrattle(&[Action::Summon {
            token: "BG_RLK_833t",
            count: 1,
        }])]),
    ),
    // Golden Fish of N'Zoth and its golden skin: each Deathrattle twice.
    (
        "TB_BaconUps_307",
        approx(EARLIER_GAINS, &[Trigger::GainsDeathrattles(2)]),
    ),
    (
        "TB_BaconShop_HP_105t_SKIN_A_G",
        approx(EARLIER_GAINS, &[Trigger::GainsDeathrattles(2)]),
    ),
    // Mishmash (a hero's Buddy): whenever your Amalgam gains stats, this
    // gains them too (golden: twice).
    (
        "TB_BaconShop_HERO_33_Buddy",
        exact(&[Trigger::GainsWithAmalgam(1)]),
    ),
    (
        "TB_BaconShop_HERO_33_Buddy_G",
        exact(&[Trigger::GainsWithAmalgam(2)]),
    ),
    // Timewarped Whirl-O-Tron: Start of Combat: copy your two left-most
    // Deathrattles, other Whirl-O-Trons' left out (golden: twice).
    (
        "BG34_Giant_599",
        approx(
            COPIED_EARLIER,
            &[Trigger::StartOfCombat(&[copy_leftmost(1)])],
        ),
    ),
    (
        "BG34_Giant_599_G",
        approx(
            COPIED_EARLIER,
            &[Trigger::StartOfCombat(&[copy_leftmost(2)])],
        ),
    ),
];

/// The Amalgam (a hero power's minion) and its skins, golden ones too.
pub const AMALGAMS: &[&str] = &[
    "TB_BaconShop_HP_033t",
    "TB_BaconShop_HP_033t_G",
    "TB_BaconShop_HP_033t_SKIN_A",
    "TB_BaconShop_HP_033t_SKIN_A_G",
    "TB_BaconShop_HP_033t_SKIN_B",
    "TB_BaconShop_HP_033t_SKIN_B_G",
    "TB_BaconShop_HP_033t_SKIN_D",
    "TB_BaconShop_HP_033t_SKIN_D_G",
];
