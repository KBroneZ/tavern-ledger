//! Card rules, keyed by exact card id (golden cards have their own id).
//!
//! Every card the engine knows is in one of two lists:
//! - [`NO_COMBAT_EFFECT`]: its text never changes a combat (shop effects,
//!   cards added to the hand, effects on the teammate or the Tavern, or
//!   only keywords, which the board already shows);
//! - [`EFFECTS`]: what it does in combat, built from the engine's
//!   primitives, and whether that is exact or an approximation.
//!
//! A card in neither list is "not modelled": the simulator says so instead of
//! a number (or, when asked, plays it as a plain minion and flags the result).
//!
//! Written from the card texts in HearthstoneJSON (D-038) and the rules in
//! design notes; no simulator code was read (clean
//! room, project rules). Effects carry only numbers and token ids, never
//! card text or art.

use crate::model::{Keywords, Tribes};

/// How closely the engine plays a card.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fidelity {
    Exact,
    /// Played, but with a known gap; the reason is shown with the result.
    Approximate(&'static str),
}

/// Something a card does. "Friendly" means the card's own side; `tribe`
/// `Tribes::NONE` means any minion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Summon `count` copies of a token, to the right of the card (or in
    /// its place when it died).
    Summon { token: &'static str, count: u8 },
    /// Give friendly minions stats; `others` leaves the card itself out.
    BuffAll {
        attack: i32,
        health: i32,
        tribe: Tribes,
        others: bool,
    },
    /// Give `count` different random friendly minions (not the card) stats.
    BuffRandom {
        attack: i32,
        health: i32,
        tribe: Tribes,
        count: u8,
    },
    /// The card gains stats.
    GainStats { attack: i32, health: i32 },
    /// The card gains a keyword.
    GainKeyword(Keywords),
    /// Give `count` different random friendly minions (not the card) that
    /// lack the keyword that keyword.
    GiveKeyword {
        keyword: Keywords,
        tribe: Tribes,
        count: u8,
    },
    /// Deal damage equal to the card's Attack to the attack's target and one
    /// random minion next to it.
    DamageTargetAndAdjacent,
    /// Deal damage equal to the card's Attack to the attack's target and both
    /// minions next to it.
    DamageTargetAndNeighbours,
    /// The card gains the target's Attack, `times` over.
    GainTargetAttack { times: i32 },
    /// Give the minion that is attacking stats (for "whenever another
    /// friendly minion attacks").
    BuffAttacker { attack: i32, health: i32 },
    /// Remove keywords from the attack's target.
    StripTarget(Keywords),
    /// Destroy the enemy minion that last damaged the card (Divine Shield
    /// does not stop it).
    DestroyKiller,
    /// Give friendly minions that have `keyword` stats.
    BuffAllWithKeyword {
        attack: i32,
        health: i32,
        keyword: Keywords,
    },
    /// Summon one token, which attacks at once (a random target, as usual).
    SummonAndAttack { token: &'static str },
    /// For each minion type, give one random friendly minion of that type
    /// stats (the card itself can be picked).
    BuffOnePerTribe { attack: i32, health: i32 },
    /// After a Reborn: give the right-most friendly minion of `tribe` stats
    /// equal to the reborn minion's Attack, `times` over.
    GiveRebornAttackToRightmost { tribe: Tribes, times: i32 },
    /// Multiply the Attack of the minion the trigger is about (the one just
    /// summoned) by `factor`.
    MultiplyAttackerAttack { factor: i32 },
    /// Summon one token with the card's Attack and Health (its full Health,
    /// not what is left), `factor` times over; with `attack_first`, the token
    /// attacks the attack's target at once (round 4).
    SummonWithStats {
        token: &'static str,
        factor: i32,
        attack_first: bool,
    },
    /// Give every friendly minion that is one of `cards` stats (round 4).
    BuffCards {
        cards: &'static [&'static str],
        attack: i32,
        health: i32,
    },
    /// Deal damage to the minions next to the attack's target (round 4).
    DamageTargetNeighbours(i32),
    /// Trigger the Deathrattles of a minion next to the card (both with
    /// `both`), except a card that does this itself (round 4).
    TriggerAdjacentDeathrattle { both: bool },
    /// Gain the Deathrattles of the `count` left-most friendly minions that
    /// have one (not other cards that do this), `times` over (round 5).
    CopyLeftmostDeathrattles { count: u8, times: u8 },
    /// Give this minion's Attack, its full Health or both to another random
    /// friendly minion (Dark Gifts, `crate::gifts`).
    GiveOwnStats { attack: bool, health: bool },
    /// Multiply this minion's Attack and Health (Dark Gifts).
    MultiplyStats { attack: i32, health: i32 },
    /// Gain the Attack of the minion to the left (Dark Gifts).
    GainLeftAttack,
    /// Trigger this minion's own Deathrattles (Dark Gifts).
    TriggerOwnDeathrattles,
    /// Summon a copy of this minion, its gifts included, to its right (Dark
    /// Gifts).
    SummonCopy,
    /// Play `count` Blood Gems on `to`: +1/+1 each plus the side's upgrades
    /// ([`crate::model::GameValues::blood_gem`]; T-609).
    Gems { to: GemTo, count: i32 },
    /// Upgrade the side's Blood Gems for the rest of the game (T-609).
    UpgradeGems { attack: i32, health: i32 },
    /// Cast a Tavern spell made of these buffs: each gives the side's spell
    /// upgrades on top ([`crate::model::GameValues::spell`]; T-609).
    Spell(&'static [Action]),
}

/// Where Blood Gems go.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GemTo {
    /// The card itself.
    Itself,
    /// Every other friendly minion.
    Others,
    /// The minion that is attacking (an attack trigger).
    Attacker,
}

impl Action {
    /// The token this action summons, if any.
    pub fn token(&self) -> Option<&'static str> {
        match *self {
            Action::Summon { token, .. }
            | Action::SummonAndAttack { token }
            | Action::SummonWithStats { token, .. } => Some(token),
            _ => None,
        }
    }
}

/// When a card acts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trigger {
    Deathrattle(&'static [Action]),
    /// Whenever this attacks, before the damage.
    Rally(&'static [Action]),
    /// After `n` friendly minions died (counted from the start of combat).
    Avenge(u8, &'static [Action]),
    StartOfCombat(&'static [Action]),
    /// A start-of-combat effect the engine does not play. Harmless when the
    /// board is taken after start of combat (the logged boards are).
    StartOfCombatUnmodelled,
    /// "Start of Combat: attack immediately", `n` times (T-609). Played with
    /// start of combat on or off: a logged board is taken at this very
    /// attack, so no board the engine is given has had it yet. It comes after
    /// its side's other start-of-combat effects and is not one of the side's
    /// turns.
    AttackImmediately(u8),
    /// Whenever another friendly minion of `tribe` attacks, before the damage.
    FriendlyAttacks {
        tribe: Tribes,
        actions: &'static [Action],
    },
    /// After this attacks (and survives), deal `times` its Attack to the
    /// highest-Health enemy minion.
    AfterAttackHitHighestHealth {
        times: i32,
    },
    /// After this attacks and kills a minion, the excess damage goes to a
    /// random enemy minion next to it.
    ExcessToAdjacent,
    /// Also damages the minions next to its target.
    Cleave,
    /// Counts as Taunt while it has Divine Shield.
    TauntWhileDivineShield,
    /// Friendly Deathrattles trigger `n` extra times.
    ExtraDeathrattles(u8),
    /// Once its Attack reaches `n`, gains Divine Shield (once).
    ShieldAtAttack(i32),
    /// Whenever a friendly minion of `tribe` attacks, this one included,
    /// before the damage.
    WheneverFriendlyAttacks {
        tribe: Tribes,
        actions: &'static [Action],
    },
    /// After a friendly minion comes back with Reborn.
    AfterFriendlyReborn(&'static [Action]),
    /// After another friendly minion of `tribe` deals damage by attacking or
    /// being attacked.
    AfterFriendlyDealsDamage {
        tribe: Tribes,
        actions: &'static [Action],
    },
    /// After a friendly minion with a Rally attacks, trigger the left-most
    /// friendly Deathrattle `times` times.
    AfterFriendlyRallyAttacks {
        times: u8,
    },
    /// After each friendly Deathrattle that triggers (round 3).
    FriendlyDeathrattle(&'static [Action]),
    /// After a friendly minion of `tribe` is summoned (Reborn included); the
    /// actions' "attacker" is the summoned minion (round 3).
    AfterFriendlySummon {
        tribe: Tribes,
        actions: &'static [Action],
    },
    /// After a friendly minion attacks, this one included (round 4).
    AfterFriendlyAttack(&'static [Action]),
    /// After this attacks and kills a minion, the excess damage goes to both
    /// enemy minions next to it (round 4).
    ExcessToBothAdjacent,
    /// After another friendly minion with a Deathrattle dies, this gains its
    /// Deathrattles for the rest of the combat, `n` times over (round 4).
    GainsDeathrattles(u8),
    /// Whenever a friendly Amalgam ([`AMALGAMS`]) gains stats, this gains
    /// them `n` times over (round 5).
    GainsWithAmalgam(i32),
    /// Its Divine Shield takes `n` hits to break (Dark Gifts).
    ShieldTakesHits(u8),
    /// Takes no damage from the minion it attacks (Dark Gifts).
    ImmuneWhileAttacking,
    /// Comes back with Reborn with its Attack and full Health and its
    /// keywords, not as a fresh card (Dark Gifts).
    RebornWithStats,
    /// The Dragons next to this one gain twice the stats they gain in
    /// combat (golden Persistent Poet; T-609).
    DoublesAdjacentDragonGains,
}

impl Trigger {
    /// The actions this trigger runs, when it has a list.
    pub fn actions(&self) -> &'static [Action] {
        match *self {
            Trigger::Deathrattle(a)
            | Trigger::Rally(a)
            | Trigger::Avenge(_, a)
            | Trigger::StartOfCombat(a)
            | Trigger::AfterFriendlyReborn(a)
            | Trigger::FriendlyDeathrattle(a)
            | Trigger::AfterFriendlyAttack(a) => a,
            Trigger::FriendlyAttacks { actions, .. }
            | Trigger::WheneverFriendlyAttacks { actions, .. }
            | Trigger::AfterFriendlyDealsDamage { actions, .. }
            | Trigger::AfterFriendlySummon { actions, .. } => actions,
            _ => &[],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CardRules {
    pub triggers: &'static [Trigger],
    pub fidelity: Fidelity,
}

const PLAIN: CardRules = CardRules {
    triggers: &[],
    fidelity: Fidelity::Exact,
};

/// The rules for a card id. `None`: not modelled.
pub fn rules(card_id: &str) -> Option<CardRules> {
    let all = EFFECTS
        .iter()
        .chain(crate::cards_round2::EFFECTS_2)
        .chain(crate::cards_round3::EFFECTS_3)
        .chain(crate::cards_round4::EFFECTS_4)
        .chain(crate::cards_round5::EFFECTS_5)
        .chain(crate::cards_round7::EFFECTS_7);
    if let Some((_, rules)) = all.into_iter().find(|(id, _)| *id == card_id) {
        return Some(*rules);
    }
    NO_COMBAT_EFFECT.contains(&card_id).then_some(PLAIN)
}

/// True when the card acts at the start of combat (played or not). A board
/// taken in the shop has not had those effects yet. "Attack immediately" is
/// left out: the engine plays it on every board.
pub fn has_start_of_combat(card_id: &str) -> bool {
    rules(card_id).is_some_and(|r| {
        r.triggers.iter().any(|t| {
            matches!(
                t,
                Trigger::StartOfCombat(_) | Trigger::StartOfCombatUnmodelled
            )
        })
    })
}

/// A minion summoned in combat. Its stats are the card's own: buffs the game
/// keeps for the whole game ("your Beetles have +2/+1") are not in the log,
/// which is why the cards that summon those are marked approximate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Token {
    pub id: &'static str,
    pub attack: i32,
    pub health: i32,
    pub keywords: Keywords,
    pub tribes: Tribes,
    pub tier: u8,
    pub golden: bool,
}

pub fn token(id: &str) -> Option<&'static Token> {
    TOKENS.iter().find(|t| t.id == id)
}

const fn tok(id: &'static str, attack: i32, health: i32, tribes: Tribes, tier: u8) -> Token {
    Token {
        id,
        attack,
        health,
        keywords: Keywords::NONE,
        tribes,
        tier,
        golden: false,
    }
}

const fn kw(t: Token, keywords: Keywords) -> Token {
    Token { keywords, ..t }
}

const fn gold(t: Token) -> Token {
    Token { golden: true, ..t }
}

pub const TOKENS: &[Token] = &[
    tok("BG_BOT_312t", 1, 1, Tribes::MECH, 1),
    gold(tok("TB_BaconUps_032t", 2, 2, Tribes::MECH, 1)),
    tok("BG_ICC_026t", 1, 1, Tribes::UNDEAD, 1),
    tok("BG28_603t", 2, 2, Tribes::BEAST, 1),
    kw(tok("BG25_010t", 2, 1, Tribes::UNDEAD, 1), Keywords::REBORN),
    kw(
        tok("BGFYM_002t", 0, 2, Tribes::ABERRATION, 1),
        Keywords::TAUNT,
    ),
    tok("BG36_200t", 1, 1, Tribes::BEAST, 1),
    tok("BG36_202", 2, 1, Tribes::BEAST, 3),
    gold(tok("BG36_202_G", 4, 2, Tribes::BEAST, 3)),
    tok("BG25_008", 4, 2, Tribes::UNDEAD, 2),
    gold(tok("BG25_008_G", 8, 4, Tribes::UNDEAD, 2)),
    // Round 4: the Golem "with this minion's stats" (its stats are set when
    // summoned; the card id is the one the user's log shows), and the
    // Carrier's Interceptors with the stats the Carrier's text gives.
    tok("BG30_MagicItem_442t", 1, 1, Tribes::NONE, 1),
    tok("BG31_HERO_802pt1t", 7, 7, Tribes::MECH, 1),
    gold(tok("BG31_HERO_802pt1t_G", 14, 14, Tribes::MECH, 1)),
    // Round 5: Foul Egg's Chicken (Foul Fowl), an Undead and a Beast in the
    // card data.
    tok("BG_RLK_833t", 3, 3, Tribes::UNDEAD.union(Tribes::BEAST), 1),
    // The Golemancy gift's Golem (the user's logs); its stats are set when
    // summoned.
    tok("BG29_864t", 5, 5, Tribes::NONE, 1),
];

pub(crate) const fn exact(triggers: &'static [Trigger]) -> CardRules {
    CardRules {
        triggers,
        fidelity: Fidelity::Exact,
    }
}

pub(crate) const fn approx(reason: &'static str, triggers: &'static [Trigger]) -> CardRules {
    CardRules {
        triggers,
        fidelity: Fidelity::Approximate(reason),
    }
}

/// Round 1; later rounds are in `cards_round2` to `cards_round5`.
pub use crate::cards_round1::EFFECTS;
pub use crate::cards_round5::{AMALGAMS, HAND_SUMMONERS};

/// Cards whose text never changes a combat, checked one by one against the
/// card data. Keywords they have are on the board already.
pub const NO_COMBAT_EFFECT: &[&str] = &[
    // Keywords only, or no text.
    "BG28_603t",
    "BG26_361",
    "BG_BOT_911",
    "BGS_119",
    "BGFYM_002t",
    "BG25_010_Gt",
    "BG25_010t",
    "BG25_001",
    "BGS_034",
    "BG32_236",
    "BGS_131",
    "BG_BOT_312t",
    "TB_BaconUps_032t",
    "BG_ICC_026t",
    "BG_ICC_026t_G",
    "BG36_200t",
    "BG36_200_Gt",
    "BG28_603t_G",
    "BGFYM_002t_G",
    // Shop, hand, Tavern or teammate effects only.
    "BG36_506",
    "BG33_830",
    "BG33_830_G",
    "TB_BaconUps_109",
    "TB_BaconUps_089",
    "TB_BaconUps_079",
    "TB_BaconUps_045",
    "BG_LOE_077",
    "BGS_116",
    "BGS_115",
    "BGS_041",
    "BGS_020",
    "BGS_004",
    "BGFYM_005",
    "BGFYM_005_G",
    "BGDUO_700",
    "BGDUO_700_G",
    "BGDUO_121",
    "BGDUO_119",
    "BGDUO_118",
    "BGDUO_118_G",
    "BGDUO_115",
    "BGDUO_114",
    "BGDUO_114_G",
    "BGDUO_111",
    "BGDUO_110",
    "BGDUO_110_G",
    "BGDUO_109",
    "BGDUO_109_G",
    "BGDUO_108",
    "BGDUO_108_G",
    "BGDUO_107",
    "BGDUO_104",
    "BGDUO_104_G",
    "BGDUO_112",
    "BGDUO_100",
    "BGDUO_120",
    "BGDUO33_150",
    "BGDUO33_150_G",
    "BGDUO33_140",
    "BGDUO33_140_G",
    "BGDUO31_207",
    "BGDUO31_207_G",
    "BGDUO31_205",
    "BGDUO31_205_G",
    "BGDUO31_203",
    "BGDUO31_202",
    "BGDUO31_201",
    "BGDUO31_201_G",
    "BG36_854",
    "BG36_854_G",
    "BG36_853",
    "BG36_853_G",
    "BG36_851",
    "BG36_764",
    "BG36_764_G",
    "BG36_763",
    "BG36_763_G",
    "BG36_762",
    "BG36_762_G",
    "BG36_733",
    "BG36_731",
    "BG36_731_G",
    "BG36_730",
    "BG36_730_G",
    "BG36_704",
    "BG36_703",
    "BG36_703_G",
    "BG36_701",
    "BG36_701_G",
    "BG36_700",
    "BG36_640",
    "BG36_640_G",
    "BG36_621",
    "BG36_621_G",
    "BG36_524",
    "BG36_524_G",
    "BG36_523",
    "BG36_523_G",
    "BG36_521",
    "BG36_521_G",
    "BG36_520",
    "BG36_511",
    "BG36_511_G",
    "BG36_503",
    "BG36_503_G",
    "BG36_372",
    "BG36_370",
    "BG36_369",
    "BG36_367",
    "BG36_367_G",
    "BG36_366",
    "BG36_362",
    "BG36_362_G",
    "BG36_360t3",
    "BG36_360t5",
    "BG36_360t6",
    "BG36_356",
    "BG36_356_G",
    "BG36_354",
    "BG36_354_G",
    "BG36_352",
    "BG36_352_G",
    "BG36_346",
    "BG36_346_G",
    "BG36_345",
    "BG36_345_G",
    "BG36_344",
    "BG36_344_G",
    "BG36_343",
    "BG36_342",
    "BG36_342_G",
    "BG36_341",
    "BG36_332",
    "BG36_332_G",
    "BG36_331",
    "BG36_331_G",
    "BG36_330",
    "BG36_330_G",
    "BG36_320",
    "BG36_312",
    "BG36_312_G",
    "BG36_311",
    "BG36_311_G",
    "BG36_300",
    "BG36_300_G",
    "BG36_243",
    "BG36_243_G",
    "BG36_242",
    "BG36_242_G",
    "BG36_240",
    "BG36_240_G",
    "BG36_204",
    "BG36_204_G",
    "BG36_201",
    "BG36_201_G",
    "BG36_180",
    "BG36_180_G",
    "BG36_115",
    "BG36_115_G",
    "BG36_114",
    "BG36_114_G",
    "BG36_112",
    "BG36_112_G",
    "BG36_111",
    "BG36_110",
    "BG36_110_G",
    "BG36_109",
    "BG36_109_G",
    "BG36_108",
    "BG36_108_G",
    "BG36_106",
    "BG36_106_G",
    "BG36_104",
    "BG36_104_G",
    "BG36_103",
    "BG36_103_G",
    "BG36_101",
    "BG36_101_G",
    "BG36_100",
    "BG36_099",
    "BG36_099_G",
    "BG36_098",
    "BG36_097",
    "BG36_097_G",
    "BG35_883",
    "BG35_883_G",
    "BG35_882",
    "BG35_881",
    "BG35_881_G",
    "BG35_155",
    "BG35_155_G",
    "BG35_150",
    "BG35_123",
    "BG34_PreMadeChamp_200",
    "BG34_Giant_602",
    "BG34_Giant_067",
    "BG34_Giant_208",
    "BG34_950",
    "BG34_950_G",
    "BG34_865",
    "BG34_865_G",
    "BG34_858",
    "BG34_858_G",
    "BG34_856",
    "BG34_692",
    "BG34_692_G",
    "BG34_684",
    "BG34_684_G",
    "BG34_682",
    "BG34_682_G",
    "BG34_637t",
    "BG34_636t",
    "BG34_636_Gt",
    "BG34_635t",
    "BG34_634t",
    "BG34_633",
    "BG34_633_G",
    "BG34_319",
    "BG34_171",
    "BG34_171_G",
    "BG34_170",
    "BG34_170_G",
    "BG34_170t",
    "BG34_170t2",
    "BG34_170t3",
    "BG34_145",
    "BG34_145_G",
    "BG33_924",
    "BG33_924_G",
    "BG33_891",
    "BG33_890t",
    "BG33_825",
    "BG33_825_G",
    "BG33_823",
    "BG33_823_G",
    "BG33_822",
    "BG33_822_G",
    "BG33_821",
    "BG32_880",
    "BG32_880_G",
    "BG32_873",
    "BG32_873_G",
    "BG32_860",
    "BG32_821",
    "BG32_821_G",
    "BG32_820",
    "BG32_820_G",
    "BG32_341",
    "BG32_341_G",
    "BG32_340",
    "BG32_336",
    "BG32_336_G",
    "BG32_330",
    "BG32_330_G",
    "BG32_324",
    "BG32_324_G",
    "BG32_237",
    "BG32_235",
    "BG32_231",
    "BG32_231_G",
    "BG32_111",
    "BG31_HERO_003_Buddy",
    "BG31_826",
    "BG31_826_G",
    "BG31_818",
    "BG31_816",
    "BG31_815",
    "BG31_815_G",
    "BG31_330",
    "BG31_330_G",
    "BG31_327",
    "BG31_327_G",
    "BG31_326",
    "BG31_326_G",
    "BG31_323",
    "BG31_323_G",
    "BG31_320",
    "BG31_320_G",
    "BG31_177",
    "BG31_177_G",
    "BG31_149",
    "BG31_149_G",
    "BG31_148",
    "BG30_123",
    "BG30_123_G",
    "BG30_121",
    "BG30_121_G",
    "BG29_813",
    "BG29_300",
    "BG29_300_G",
    "BG28_741",
    "BG28_707",
    "BG28_707_G",
    "BG28_633",
    "BG28_633_G",
    "BG28_582",
    "BG28_582_G",
    "BG28_550",
    "BG28_303_G",
    "BG27_HERO_801_Buddy",
    "BG27_084",
    "BG27_084_G",
    "BG27_016",
    "BG27_005",
    "BG27_005_G",
    "BG27_000",
    "BG26_LOOT_534",
    "BG26_ICC_901",
    "BG26_ICC_901_G",
    "BG26_814",
    "BG26_814_G",
    "BG26_810",
    "BG26_810_G",
    "BG26_524",
    "BG26_524_G",
    "BG26_523",
    "BG26_175",
    "BG26_174",
    "BG26_174_G",
    "BG26_152",
    "BG26_149",
    "BG26_147",
    "BG26_146",
    "BG26_146_G",
    "BG26_135",
    "BG26_135_G",
    "BG25_034",
    "BG25_034_G",
    "BG25_011",
    "BG25_011_G",
    "BG24_715",
    "BG23_357",
    "BG23_017",
    "BG22_403",
    "BG22_403_G",
    "BG22_202",
    "BG21_015",
    "BG21_015_G",
    "BG21_004",
    "BG21_004_G",
    "BG20_101",
    "BG20_101_G",
    "BG20_100",
    "BG20_100_G",
    // Second pass over the not-modelled list.
    "BG34_500",
    "BG34_500_G",
    "BG34_638t",
    "BG34_638_Gt",
    "BG36_848",
    "BG36_848_G",
    "BG34_Giant_314",
    "BG25_001_G",
    "BGDUO31_212",
    "BGDUO31_212_G",
    "BGS_115t",
    "BG31_HERO_801pt",
    "BG29_864t",
    "BG36_506_G",
    "TB_BaconShop_HP_033t_SKIN_B",
    "TB_BaconShop_HP_033t",
    "TB_BaconUps_159",
    "BG28_HERO_800_Buddy",
    "BG28_HERO_800_Buddy_G",
    "BG36_369_G",
    "BG34_856_G",
    "BGDUO_100_G",
    "BG26_137",
    "BG34_Giant_082",
    "BG34_Giant_082_G",
    "BG27_016_G",
    "BG31_HERO_802pt7",
    "BG36_370_G",
    "BG24_018",
    "BG36_320_G",
    "BGDUO_117",
    "BGDUO_117_G",
    "BG26_525",
    "BGDUO_120_G",
    "BG31_176",
    "BG36_520_G",
    "BG32_235_G",
    "BG36_366_G",
    "BG34_Giant_038",
    "BG31_360",
    "BG34_Giant_034",
    "BG34_170t_G",
    "TB_BaconUps_099",
    "BG23_HERO_306_Buddy",
    "BGDUO31_211",
    "BG36_733_G",
    "BG34_697",
    "BG31_816_G",
    "BG36_372_G",
    "BG34_Giant_302",
    "BG36_181",
    "BG26_HERO_104_Buddy",
    "BG34_Giant_208_G",
    "BG34_Giant_593",
    "BG27_513",
    // The other form of cards listed above in one form only: a golden card
    // has its own id and the same text with larger numbers, and Disguised
    // Graverobber's normal form (battlecry).
    "BG28_303",
    "BG26_361_G",
    "TB_BaconUps_149",
    "BG32_236_G",
    "TB_BaconUps_251",
    "TB_BaconUps_167",
    "TB_BaconUps_156",
    "BGDUO_121_G",
    "BGDUO_119_G",
    "BGDUO_115_G",
    "BGDUO_111_G",
    "BGDUO_107_G",
    "BGDUO_112_G",
    "BGDUO31_203_G",
    "BGDUO31_202_G",
    "BG36_851_G",
    "BG36_704_G",
    "BG36_700_G",
    "BG36_360_Gt3",
    "BG36_360_Gt5",
    "BG36_360_Gt6",
    "BG36_343_G",
    "BG36_341_G",
    "BG36_111_G",
    "BG36_100_G",
    "BG36_098_G",
    "BG35_882_G",
    "BG35_150_G",
    "BG35_123_G",
    "BG34_PreMadeChamp_200_G",
    "BG34_Giant_602_G",
    "BG34_Giant_067_G",
    "BG34_637_Gt",
    "BG34_635_Gt",
    "BG34_634_Gt",
    "BG34_319_G",
    "BG34_170t2_G",
    "BG34_170t3_G",
    "BG33_891_G",
    "BG33_821_G",
    "BG32_860_G",
    "BG32_340_G",
    "BG32_237_G",
    "BG32_111_G",
    "BG31_HERO_003_Buddy_G",
    "BG31_818_G",
    "BG31_148_G",
    "BG28_741_G",
    "BG28_550_G",
    "BG27_HERO_801_Buddy_G",
    "BG27_000_G",
    "BG26_523_G",
    "BG26_175_G",
    "BG26_152_G",
    "BG26_149_G",
    "BG26_147_G",
    "BG24_715_G",
    "BG23_357_G",
    "BG23_017_G",
    "BG22_202_G",
    "BG34_Giant_314_G",
    "BGS_115t_G",
    "BG31_HERO_801pt_G",
    "BG29_864_Gt",
    "BG26_137_G",
    "BG31_HERO_802pt7_G",
    "BG24_018_G",
    "BG26_525_G",
    "BG31_176_G",
    "BG34_Giant_038_G",
    "BG31_360_G",
    "BG34_Giant_034_G",
    "BG23_HERO_306_Buddy_G",
    "BGDUO31_211_G",
    "BG34_697_G",
    "BG34_Giant_302_G",
    "BG36_181_G",
    "BG26_HERO_104_Buddy_G",
    "BG34_Giant_593_G",
    "BG27_513_G",
];

#[cfg(test)]
#[path = "cards_tests.rs"]
mod tests;
