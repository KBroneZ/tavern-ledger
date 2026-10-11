//! Card coverage, round 4 (T-604, session 075): the cards that blocked most
//! of the user's combats after round 3 and that the board alone can play,
//! from their card texts in HearthstoneJSON (D-038). Same rules as
//! [`crate::cards`]: exact card ids, golden cards written out, approximations
//! named with the reason.
//!
//! Left blocking after round 4, and why: the hand summoners (`BG34_140`, `BG31_835`,
//! `BG21_HERO_030t`, and `BG34_Giant_017`, which summons a minion from the
//! hand too: the hand is not seen); `BG31_999` (its Deathrattle copies the
//! minion it destroyed at the start of combat, which no board the app has
//! still shows); `BGS_008` and `BG34_Giant_590` (random minions from a pool);
//! `BG34_Giant_599` (copies Deathrattles at the start of combat, so its own
//! are not on the board); `TB_BaconShop_HERO_33_Buddy` (follows the Amalgam,
//! a hero power minion); the golden Fish of N'Zoth skin (gains each
//! Deathrattle twice). Round 5 (`cards_round5`) took up all of these but
//! `BG31_999`, `BGS_008` and `BG34_Giant_590`.

use crate::cards::{approx, exact, Action, CardRules, Trigger};
use crate::model::{Keywords, Tribes};

/// The Golem "with this minion's stats": the token the user's log shows for
/// Jailbird Juggernaut; the card data has no Golem token of its own.
const GOLEM: &str = "BG30_MagicItem_442t";
const BEETLES: &[&str] = &["BG28_603t", "BG28_603t_G"];

const GOLEM_ID: &str = "its Golem's card id is the one the log shows for Jailbird Juggernaut";
const GOLEM_FIRST: &str =
    "its Golem's card id is from the log; what the card does when its Golem kills the target is not checked";
const EARLIER_GAINS: &str = "Deathrattles it gained in earlier combats are not on the board";
const BEETLE_BUFFS: &str =
    "Beetles' game-long buffs are not in the log, and Beetles summoned later miss this combat's";
const NEIGHBOURS: &str =
    "which neighbour it picks, and neighbours that die with it, are not checked";
const BOTH_NEIGHBOURS: &str = "neighbours that die with it are not checked";
const SCRIBES: &str = "Scribes that die in this combat do not buff it yet";
const PROTOSS: &str = "it improves during the game; the card's printed numbers are used";
const KEYWORD_PICK: &str = "picks only minions that miss one of the keywords";
const BLOOD_GEMS: &str = "Blood Gems are played as +1/+1; upgrades are not in the log";

const fn golem(factor: i32, attack_first: bool) -> Action {
    Action::SummonWithStats {
        token: GOLEM,
        factor,
        attack_first,
    }
}

const fn beetles(attack: i32, health: i32) -> Action {
    Action::BuffCards {
        cards: BEETLES,
        attack,
        health,
    }
}

const fn summon(token: &'static str, count: u8) -> Action {
    Action::Summon { token, count }
}

const SPIRIT_KEYWORDS: Keywords = Keywords::WINDFURY
    .union(Keywords::DIVINE_SHIELD)
    .union(Keywords::TAUNT);

const fn spirit(count: u8) -> Action {
    Action::GiveKeyword {
        keyword: SPIRIT_KEYWORDS,
        tribe: Tribes::NONE,
        count,
    }
}

pub const EFFECTS_4: &[(&str, CardRules)] = &[
    // Dark Paradox (Golem Dark Gift): Deathrattle: summon a Golem with this
    // minion's stats (golden: double).
    (
        "BG36_360t9",
        approx(GOLEM_ID, &[Trigger::Deathrattle(&[golem(1, false)])]),
    ),
    (
        "BG36_360_Gt9",
        approx(GOLEM_ID, &[Trigger::Deathrattle(&[golem(2, false)])]),
    ),
    // Jailbird Juggernaut: Rally: summon a Golem with this minion's stats
    // (golden: double) to attack the target first.
    (
        "BG36_333",
        approx(GOLEM_FIRST, &[Trigger::Rally(&[golem(1, true)])]),
    ),
    (
        "BG36_333_G",
        approx(GOLEM_FIRST, &[Trigger::Rally(&[golem(2, true)])]),
    ),
    // Fish of N'Zoth (a hero power's minion) and its skin.
    (
        "TB_BaconShop_HP_105t",
        approx(EARLIER_GAINS, &[Trigger::GainsDeathrattles(1)]),
    ),
    (
        "TB_BaconShop_HP_105t_SKIN_A",
        approx(EARLIER_GAINS, &[Trigger::GainsDeathrattles(1)]),
    ),
    // Ravaging Scorpid: after a friendly minion attacks, your Beetles have
    // +4/+4 (golden +8/+8); Deathrattle: summon a 2/2 Beetle (golden two).
    (
        "BG36_209",
        approx(
            BEETLE_BUFFS,
            &[
                Trigger::AfterFriendlyAttack(&[beetles(4, 4)]),
                Trigger::Deathrattle(&[summon("BG28_603t", 1)]),
            ],
        ),
    ),
    (
        "BG36_209_G",
        approx(
            BEETLE_BUFFS,
            &[
                Trigger::AfterFriendlyAttack(&[beetles(8, 8)]),
                Trigger::Deathrattle(&[summon("BG28_603t", 2)]),
            ],
        ),
    ),
    // Timewarped Warghoul: Deathrattle: trigger an adjacent minion's
    // Deathrattle (golden: both); Taunt is on the board.
    (
        "BG34_Giant_331",
        approx(
            NEIGHBOURS,
            &[Trigger::Deathrattle(&[
                Action::TriggerAdjacentDeathrattle { both: false },
            ])],
        ),
    ),
    (
        "BG34_Giant_331_G",
        approx(
            BOTH_NEIGHBOURS,
            &[Trigger::Deathrattle(&[
                Action::TriggerAdjacentDeathrattle { both: true },
            ])],
        ),
    ),
    // Golden Choral Mrrrglr and Diremuck Forager: start of combat only, from
    // the hand, already on a board taken after it (as their normal cards).
    ("BG26_354_G", exact(&[Trigger::StartOfCombatUnmodelled])),
    ("BG27_556_G", exact(&[Trigger::StartOfCombatUnmodelled])),
    // San'layn Scribe: its stats count the Scribes dead before this combat.
    ("BGDUO31_208", approx(SCRIBES, &[])),
    ("BGDUO31_208_G", approx(SCRIBES, &[])),
    // Carrier (a hero power's minion): Avenge (4): summon a 7/7 Interceptor
    // (golden 14/14), then improve.
    (
        "BG31_HERO_802pt1",
        approx(
            PROTOSS,
            &[Trigger::Avenge(4, &[summon("BG31_HERO_802pt1t", 1)])],
        ),
    ),
    (
        "BG31_HERO_802pt1_G",
        approx(
            PROTOSS,
            &[Trigger::Avenge(4, &[summon("BG31_HERO_802pt1t_G", 1)])],
        ),
    ),
    // Colossus: Rally: deal 2 damage (golden 4) to the target's neighbours.
    (
        "BG31_HERO_802pt",
        approx(
            PROTOSS,
            &[Trigger::Rally(&[Action::DamageTargetNeighbours(2)])],
        ),
    ),
    (
        "BG31_HERO_802pt_G",
        approx(
            PROTOSS,
            &[Trigger::Rally(&[Action::DamageTargetNeighbours(4)])],
        ),
    ),
    // Golden Wildfire Elemental: excess damage to both adjacent enemies.
    ("TB_BaconUps_166", exact(&[Trigger::ExcessToBothAdjacent])),
    // Spirit of Air: Deathrattle: give a random friendly minion (golden: 2)
    // Windfury, Divine Shield and Taunt.
    (
        "TB_BaconShop_HERO_76_Buddy",
        approx(KEYWORD_PICK, &[Trigger::Deathrattle(&[spirit(1)])]),
    ),
    (
        "TB_BaconShop_HERO_76_Buddy_G",
        approx(KEYWORD_PICK, &[Trigger::Deathrattle(&[spirit(2)])]),
    ),
    // Golden Prodigious Tusker: two Blood Gems on each other attacker.
    (
        "BG33_430_G",
        approx(
            BLOOD_GEMS,
            &[Trigger::FriendlyAttacks {
                tribe: Tribes::NONE,
                actions: &[Action::BuffAttacker {
                    attack: 2,
                    health: 2,
                }],
            }],
        ),
    ),
];
