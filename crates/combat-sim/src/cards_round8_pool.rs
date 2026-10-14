//! The minions Ghastcoiler can summon (T-616): every Battlegrounds minion with
//! a Deathrattle in HearthstoneJSON's pool list, except Ghastcoiler itself,
//! with the card's printed stats, keywords and types. Generated from the
//! card data once; the engine keeps only ids and numbers (D-038).

use crate::cards::{kw, tok, Token};
use crate::model::{Keywords, Tribes};

/// Card ids Ghastcoiler picks from, each as likely as the others.
pub const DEATHRATTLE_POOL: &[&str] = &[
    "BG28_300",
    "BG29_611",
    "BG31_803",
    "BG25_022",
    "BG31_801",
    "BG36_116",
    "BGDUO_111",
    "BG25_010",
    "BG28_309",
    "BG30_125",
    "BG34_856",
    "BG36_101",
    "BG36_113",
    "BG36_202",
    "BG36_521",
    "BG36_730",
    "BG36_854",
    "BGDUO31_207",
    "BG32_336",
    "BG32_880",
    "BG34_171",
    "BG34_682",
    "BG34_690",
    "BG35_881",
    "BG36_362",
    "BG36_731",
    "BGDUO31_203",
    "BGDUO_110",
    "BG23_318",
    "BG25_009",
    "BG31_809",
    "BG31_925",
    "BG32_111",
    "BG32_820",
    "BG33_821",
    "BG33_828",
    "BG34_633",
    "BG35_604",
    "BG36_103",
    "BG36_318",
    "BGDUO_701",
    "BGS_018",
    "BG23_017",
    "BG31_835",
    "BG36_104",
    "BG36_209",
    "BGS_040",
    "BG27_016",
    "BG31_999",
    "BG34_319",
];

/// The printed facts of the pool's minions that are not tokens already.
pub(crate) const POOL_TOKENS: &[Token] = &[
    // Harmless Bonehead
    tok("BG28_300", 1, 1, Tribes::UNDEAD, 1),
    // Cord Puller
    kw(
        tok("BG29_611", 1, 1, Tribes::MECH, 1),
        Keywords::DIVINE_SHIELD,
    ),
    // Buzzing Vermin
    kw(tok("BG31_803", 1, 1, Tribes::BEAST, 1), Keywords::TAUNT),
    // Scarlet Skull
    kw(tok("BG25_022", 2, 1, Tribes::UNDEAD, 2), Keywords::REBORN),
    // Forest Rover
    tok("BG31_801", 1, 1, Tribes::BEAST, 2),
    // Underrot Spawn
    tok("BG36_116", 2, 2, Tribes::ABERRATION, 2),
    // Generous Geomancer
    tok("BGDUO_111", 1, 1, Tribes::QUILBOAR, 2),
    // Handless Forsaken
    tok("BG25_010", 2, 1, Tribes::UNDEAD, 3),
    // Mummifier
    tok("BG28_309", 5, 2, Tribes::UNDEAD, 3),
    // Cadaver Caretaker
    tok("BG30_125", 3, 3, Tribes::UNDEAD, 3),
    // Waveling
    tok("BG34_856", 5, 1, Tribes::ELEMENTAL, 3),
    // Unwilling Slacker
    tok("BG36_101", 3, 2, Tribes::ABERRATION, 3),
    // Drifting Sacrifice
    kw(
        tok("BG36_113", 2, 1, Tribes::ABERRATION, 3),
        Keywords::REBORN,
    ),
    // Locked-up Mutineer
    tok("BG36_521", 6, 3, Tribes::PIRATE, 3),
    // Trapped Clapper
    tok("BG36_730", 2, 2, Tribes::DEMON, 3),
    // Rescue Bot
    kw(tok("BG36_854", 2, 1, Tribes::MECH, 3), Keywords::TAUNT),
    // Wheeled Crewmate
    tok("BGDUO31_207", 6, 3, Tribes::PIRATE, 3),
    // Gormling Gourmet
    kw(tok("BG32_336", 4, 3, Tribes::MURLOC, 4), Keywords::TAUNT),
    // Friendly Geist
    tok("BG32_880", 6, 3, Tribes::UNDEAD, 4),
    // Conveyor Construct
    tok("BG34_171", 5, 2, Tribes::MECH, 4),
    // Razorfen Flapper
    tok("BG34_682", 5, 3, Tribes::QUILBOAR, 4),
    // Plaguerunner
    tok("BG34_690", 4, 2, Tribes::UNDEAD, 4),
    // Leyline Surfacer
    tok("BG35_881", 4, 6, Tribes::ELEMENTAL, 4),
    // Sacrificial Wrathguard
    tok("BG36_362", 5, 3, Tribes::DEMON, 4),
    // Imp-lusionist
    tok("BG36_731", 4, 2, Tribes::DEMON, 4),
    // Shifty Snake
    tok("BGDUO31_203", 6, 1, Tribes::BEAST, 4),
    // Feisty Freshwater
    tok("BGDUO_110", 6, 4, Tribes::ELEMENTAL, 4),
    // Leeroy the Reckless
    tok("BG23_318", 6, 2, Tribes::NONE, 5),
    // Eternal Summoner
    kw(tok("BG25_009", 8, 1, Tribes::UNDEAD, 5), Keywords::REBORN),
    // Turquoise Skitterer
    tok("BG31_809", 5, 5, Tribes::BEAST, 5),
    // Showy Cyclist
    tok("BG31_925", 4, 2, Tribes::NAGA, 5),
    // Nightmare Par-tea Guest
    tok("BG32_111", 3, 3, Tribes::ALL, 5),
    // Firescale Hoarder
    tok("BG32_820", 5, 5, Tribes::NAGA.union(Tribes::DRAGON), 5),
    // Shipwrecked Rascal
    tok("BG33_821", 5, 4, Tribes::PIRATE, 5),
    // Ship Master Eudora
    tok("BG33_828", 8, 3, Tribes::PIRATE, 5),
    // Draconic Warden
    tok("BG34_633", 7, 4, Tribes::DRAGON, 5),
    // Sewer Lord
    tok("BG35_604", 4, 6, Tribes::BEAST, 5),
    // N'raqi Sapper
    tok("BG36_103", 6, 3, Tribes::ABERRATION, 5),
    // Faceless Converter
    tok("BG36_318", 5, 5, Tribes::ABERRATION, 5),
    // C'Thrax Wrecker
    tok("BGDUO_701", 8, 8, Tribes::ABERRATION, 5),
    // Goldrinn, the Great Wolf
    tok("BGS_018", 7, 7, Tribes::BEAST, 5),
    // Sanguine Champion
    tok("BG23_017", 9, 4, Tribes::QUILBOAR, 6),
    // Deathly Striker
    tok("BG31_835", 8, 8, Tribes::UNDEAD, 6),
    // Dark Puppeteer
    tok("BG36_104", 8, 4, Tribes::ABERRATION, 6),
    // Ravaging Scorpid
    tok("BG36_209", 6, 7, Tribes::BEAST, 6),
    // Nadina the Red
    tok("BGS_040", 8, 4, Tribes::NONE, 6),
    // Champion of Sargeras
    tok("BG27_016", 8, 8, Tribes::DEMON, 7),
    // Stitched Salvager
    tok("BG31_999", 16, 4, Tribes::UNDEAD, 7),
    // Highkeeper Ra
    tok("BG34_319", 6, 6, Tribes::NONE, 7),
];
