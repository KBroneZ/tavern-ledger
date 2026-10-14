//! Card coverage, round 8 (T-616): the pool minions the simulator had not
//! reviewed, from their card texts in HearthstoneJSON (D-038) and the
//! Battlegrounds wiki. Same rules as [`crate::cards`]: exact card ids, golden
//! cards written out, approximations named with the reason.
//!
//! Most of the 35 cards are Naga, Spellcraft and Tavern-spell minions whose
//! text works in the shop or the hand only: they are in
//! [`NO_COMBAT_EFFECT_8`]. The ones that act in combat are in [`EFFECTS_8`].
//! `BG31_999` and the Undead Creation cards stay refused, with the reason
//! [`refusal`] gives.

use crate::cards::{approx, exact, Action, CardRules, Trigger};
use crate::model::Tribes;

pub use crate::cards_round8_pool::DEATHRATTLE_POOL;
pub(crate) use crate::cards_round8_pool::POOL_TOKENS;

/// Why the simulator refuses a card it has reviewed and cannot play yet, for
/// the page to show (the card's id, then the reason). Both forms of a card.
pub const REFUSED: &[(&str, &str)] = &[
    ("BG31_999", SALVAGER),
    ("BG31_999_G", SALVAGER),
    ("BG34_Giant_590", UNDEAD_CREATIONS),
    ("BG34_Giant_590_G", UNDEAD_CREATIONS),
    ("BG25_HERO_100_Buddy", UNDEAD_CREATIONS),
    ("BG25_HERO_100_Buddy_G", UNDEAD_CREATIONS),
    // Cards that act in combat but are no minion on a board (build 254477):
    // the simulator is given minions only.
    ("BG30_MagicItem_952", TRINKET_START_OF_COMBAT),
    ("BG30_MagicItem_989t", TRINKET_AURA),
    ("BG_OldGod", OLD_GOD),
];

const SALVAGER: &str = "its Deathrattle copies the minion it destroys at the start of combat; the board is taken after that, so the copied minion is gone and not known";
const UNDEAD_CREATIONS: &str =
    "its Deathrattle summons random Undead Creations and the list of them is not known";
const TRINKET_START_OF_COMBAT: &str = "a trinket that acts at the start of combat; trinkets are not part of the boards the simulator is given";
const TRINKET_AURA: &str = "a trinket that gives your Undead Attack; trinkets are not part of the boards the simulator is given";
const OLD_GOD: &str = "a spell that awakens an Old God after Aberrations die in combat; it is not a minion on the board and is not played";
/// What the page says about a card nobody has looked at yet.
const NOT_REVIEWED: &str = "the simulator has not reviewed this card yet";

/// The reason a card is refused: the one written for it in [`REFUSED`], else
/// that the card has not been reviewed.
pub fn refusal(card_id: &str) -> &'static str {
    REFUSED
        .iter()
        .find(|(id, _)| *id == card_id)
        .map_or(NOT_REVIEWED, |(_, why)| why)
}

const SPELL_FOLLOWERS: &str = "a spell it casts can trigger cards that follow spells (Torrential Ruiner, Fleeing Fugitive, Abyssal Bruiser): not played";
const TENTACLE: &str = "it gains stats until next turn whenever another friendly minion gains stats in combat; that is not played";
const MAXIMUM_STATS: &str =
    "what its maximum stats are is not checked: its Attack and full Health are used";
const SPELL_COUNT: &str =
    "its bonus improves with the Tavern spells cast this game; the printed one is used";
const ANY_DEATHRATTLE: &str =
    "it picks from every Deathrattle minion, not only the minion types in this game";

const fn summon(token: &'static str, count: u8) -> Action {
    Action::Summon { token, count }
}

const KODO_1: &[Action] = &[Action::GiveSummonedOwnStats {
    factor: 1,
    times: 3,
}];
const KODO_2: &[Action] = &[Action::GiveSummonedOwnStats {
    factor: 2,
    times: 3,
}];

const fn kodo(actions: &'static [Action]) -> Trigger {
    Trigger::AfterFriendlySummon {
        tribe: Tribes::NONE,
        actions,
    }
}

const NAGAS_2_1: &[Action] = &[Action::BuffAll {
    attack: 2,
    health: 1,
    tribe: Tribes::NAGA,
    others: true,
}];
const NAGAS_4_2: &[Action] = &[Action::BuffAll {
    attack: 4,
    health: 2,
    tribe: Tribes::NAGA,
    others: true,
}];

pub const EFFECTS_8: &[(&str, CardRules)] = &[
    // Timewarped Jazzer: Deathrattle: your Blood Gems give an extra +2 Health
    // this game (golden +4); the gems played later in combat give more.
    (
        "BG34_Giant_306",
        exact(&[Trigger::Deathrattle(&[Action::UpgradeGems {
            attack: 0,
            health: 2,
        }])]),
    ),
    (
        "BG34_Giant_306_G",
        exact(&[Trigger::Deathrattle(&[Action::UpgradeGems {
            attack: 0,
            health: 4,
        }])]),
    ),
    // Dark Paradox, the tier 5 form: Divine Shield takes 3 hits (golden 4).
    ("BG36_360t4", exact(&[Trigger::ShieldTakesHits(3)])),
    ("BG36_360_Gt4", exact(&[Trigger::ShieldTakesHits(4)])),
    // Rot Hide Gnoll: +1 Attack for each friendly minion that died this
    // combat (golden +2).
    ("BG25_013", exact(&[Trigger::AttackPerFriendlyDeath(1)])),
    ("BG25_013_G", exact(&[Trigger::AttackPerFriendlyDeath(2)])),
    // Sewer Lord and its Sewer Rats: Deathrattle: two Sewer Rats that summon
    // 2/3 Turtles with Taunt (golden: Golden Sewer Rats, 4/6 Turtles).
    (
        "BG35_604",
        exact(&[Trigger::Deathrattle(&[summon("BG19_010", 2)])]),
    ),
    (
        "BG35_604_G",
        exact(&[Trigger::Deathrattle(&[summon("BG19_010_G", 2)])]),
    ),
    (
        "BG19_010",
        exact(&[Trigger::Deathrattle(&[summon("BG19_010t", 1)])]),
    ),
    (
        "BG19_010_G",
        exact(&[Trigger::Deathrattle(&[summon("BG19_010_Gt", 1)])]),
    ),
    // Stalwart Kodo: after you summon a minion in combat, give it this
    // minion's maximum stats, 3 times per combat (golden: double).
    ("BG34_322", approx(MAXIMUM_STATS, &[kodo(KODO_1)])),
    ("BG34_322_G", approx(MAXIMUM_STATS, &[kodo(KODO_2)])),
    // Showy Cyclist: Deathrattle: give all your Naga +2/+1 (golden +4/+2).
    (
        "BG31_925",
        approx(SPELL_COUNT, &[Trigger::Deathrattle(NAGAS_2_1)]),
    ),
    (
        "BG31_925_G",
        approx(SPELL_COUNT, &[Trigger::Deathrattle(NAGAS_4_2)]),
    ),
    // Ghastcoiler: Deathrattle: summon 2 random Deathrattle minions (golden 4).
    (
        "BGS_008",
        approx(
            ANY_DEATHRATTLE,
            &[Trigger::Deathrattle(&[Action::SummonRandomDeathrattle {
                count: 2,
            }])],
        ),
    ),
    (
        "TB_BaconUps_057",
        approx(
            ANY_DEATHRATTLE,
            &[Trigger::Deathrattle(&[Action::SummonRandomDeathrattle {
                count: 4,
            }])],
        ),
    ),
    // Seafloor Recruiter: Rally: cast Chef's Choice, which gets a card for the
    // hand, on the minion to the right (golden twice): nothing changes on the
    // board, but a spell cast on a Naga or on itself moves the cards that
    // follow spells.
    ("BG34_925", approx(SPELL_FOLLOWERS, &[])),
    ("BG34_925_G", approx(SPELL_FOLLOWERS, &[])),
    // Sandy (Duos): Start of Combat: transform into a copy of the teammate's
    // highest-Health minion. A board taken after start of combat already
    // shows the copy.
    ("BGDUO_125", exact(&[Trigger::StartOfCombatUnmodelled])),
    ("BGDUO_125_G", exact(&[Trigger::StartOfCombatUnmodelled])),
    // Tentacle of C'Thun (hero buddy; an Aberration since build 254477):
    // after a different friendly minion gains stats, +1/+1 until next turn
    // (golden +2/+2). Combat-time gains by the others are not tracked.
    ("TB_BaconShop_HERO_29_Buddy", approx(TENTACLE, &[])),
    ("TB_BaconShop_HERO_29_Buddy_G", approx(TENTACLE, &[])),
];

/// Cards whose text works in the shop, the hand or the Tavern only, or whose
/// bonus the board already shows (golden form of each beside it). Checked
/// one by one against the card texts (T-616).
pub const NO_COMBAT_EFFECT_8: &[&str] = &[
    // Spellcraft spells (cast from the hand in the shop; what they give
    // lasts "until next turn" and is on the board), Battlecries and
    // end-of-turn effects.
    "BG23_000",
    "BG23_000_G",
    "BG23_002",
    "BG23_002_G",
    "BG23_004",
    "BG23_004_G",
    "BG23_007",
    "BG23_007_G",
    "BG23_008",
    "BG23_008_G",
    "BG23_009",
    "BG23_009_G",
    "BG26_505",
    "BG26_505_G",
    "BG27_514",
    "BG27_514_G",
    "BG31_920",
    "BG31_920_G",
    "BG31_924",
    "BG31_924_G",
    "BG32_835",
    "BG32_835_G",
    "BG32_837",
    "BG32_837_G",
    "BG33_319",
    "BG33_319_G",
    "BGDUO31_209",
    "BGDUO31_209_G",
    "BGS_123",
    "TB_BaconUps_162",
    // Triggers of "play", "sell", "pass" or "activate": shop actions.
    "BG31_035",
    "BG31_035_G",
    "BG31_812",
    "BG31_812_G",
    "BG36_206",
    "BG36_206_G",
    "BG36_308",
    "BG36_308_G",
    "BG36_508",
    "BG36_508_G",
    "BGDUO_122",
    "BGDUO_122_G",
    // Cards that follow Tavern spells, or raise what they give: spells are cast
    // in combat only by a few cards, each marked on its own (Seafloor Recruiter).
    "BG35_341",
    "BG35_341_G",
    "BG35_921",
    "BG35_921_G",
    "BG36_622",
    "BG36_622_G",
    "BG36_921",
    "BG36_921_G",
    // Keywords only (Magnetic is a shop action; Reborn is on the board), and
    // the placeholder of Dark Paradox, whose forms are separate cards.
    "BG_DEEP_015",
    "BG_DEEP_015_G",
    "BG36_360",
    "BG36_360_G",
    // Spells and trinkets of the patch of 2026-10-14 (build 254477) whose
    // text works in the shop, the hand or on a turn's start only. None is a
    // minion, so none has a golden form: Butchering, Gem Confiscation,
    // Easterly Winds, Blood Gem Barrage; Lava Lamp, Shark Cannon, Maldraxxus
    // Dagger, Sludge Portrait.
    "BG28_604",
    "BG28_698",
    "BG34_444",
    "BG34_689",
    "BG30_MagicItem_951",
    "BG32_MagicItem_232",
    "BG36_MagicItem_370",
    "BG36_MagicItem_430",
    // Sewer Lord's Turtles: keywords only.
    "BG19_010t",
    "BG19_010_Gt",
];
