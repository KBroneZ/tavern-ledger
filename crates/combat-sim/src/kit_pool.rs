//! The rest of the Battlegrounds hero powers and trinkets in the card data
//! (HearthstoneJSON build 254477), none seen in the user's games, reviewed
//! by their text alone (T-631 group 2). The rule: "no combat effect" only
//! when the text proves it; a combat effect the engine does not play keeps
//! blocking the odds by name unless it is small and bounded, then it is
//! approximate. Names only, as comments; no card text.

/// Their text never changes a combat (the shop, the hand, the Tavern, the
/// end or start of a turn, a minion they give, played by its own card rules,
/// or a counter that pays out outside combat).
pub const NO_COMBAT_EFFECT_POOL: &[&str] = &[
    // Hero powers.
    "BG20_HERO_201p",        // Spirit Swap
    "BG20_HERO_201p2",       // Spirit Swap
    "BG20_HERO_283p_t1",     // Westfall
    "BG20_HERO_283p_t2",     // Ironforge
    "BG20_HERO_283p_t3",     // Eastern Plaguelands
    "BG22_HERO_007p",        // Azshara's Ambition
    "BG22_HERO_007p2",       // Naga Conquest
    "BG23_HERO_303p2",       // Detective for Hire
    "BG23_HERO_304p",        // Relics of the Deep
    "BG23_HERO_305p",        // The Perfect Crime
    "BG27_Anomaly_805t",     // Match Fixing
    "BG31_Anomaly_117t",     // Eject!
    "BG31_HERO_811p3",       // Ultralisk Cavern
    "BG34_HERO_002p",        // Double Time
    "BG35_Anomaly_002t",     // Mystery Cube
    "BG35_Anomaly_007t",     // Lesser Crystal Ball
    "BG35_Anomaly_008t",     // Greater Crystal Ball
    "BGDUO_Anomaly_003t",    // Golden Friendship
    "BGDUO_Anomaly_007t",    // Pooled Resources
    "BGDUO_HERO_222p",       // Double Trouble
    "BGDUO_HERO_223p",       // Double Trouble
    "TB_BaconShop_HP_009",   // Skilled Bartender
    "TB_BaconShop_HP_018",   // Rage Potion
    "TB_BaconShop_HP_036t",  // Nether Portal
    "TB_BaconShop_HP_050",   // Banshee's Blessing
    "TB_BaconShop_HP_051",   // Honorable Warband
    "TB_BaconShop_HP_057",   // Adventure!
    "TB_BaconShop_HP_062",   // Dream Portal
    "TB_BaconShop_HP_065",   // Demon Hunter Training
    "TB_BaconShop_HP_065t2", // Spectral Sight
    "TB_BaconShop_HP_068",   // Imprison
    "TB_BaconShop_HP_085",   // Tavern Lighting
    "TB_BaconShop_HP_087",   // BUY, INSECT!
    "TB_BaconShop_HP_087t",  // Sulfuras
    "TB_BaconShop_HP_088",   // Avalanche
    "TB_BaconShop_HP_702",   // Runic Empowerment
    // Trinkets.
    "BG30_MagicItem_402",   // Conductor Portrait
    "BG30_MagicItem_414",   // Kodo Leather Pouch
    "BG30_MagicItem_414t",  // Kodo Leather Pouch
    "BG30_MagicItem_419",   // Exquisite Dishware
    "BG30_MagicItem_429",   // Demonblood Gourd
    "BG30_MagicItem_434",   // Replica Cathedral
    "BG30_MagicItem_543",   // Devourer Sticker
    "BG30_MagicItem_544",   // Nomi Sticker
    "BG30_MagicItem_545",   // Fridge Magnet
    "BG30_MagicItem_548",   // Glowscale Portrait
    "BG30_MagicItem_701",   // The Eye of Sargeras
    "BG30_MagicItem_702",   // Primalfin Portrait
    "BG30_MagicItem_709",   // Electromagnetic Device
    "BG30_MagicItem_709t",  // Electromagnetic Device
    "BG30_MagicItem_710",   // Fungalmancer Sticker
    "BG30_MagicItem_711",   // Marine Signet
    "BG30_MagicItem_714",   // Precious Pearl
    "BG30_MagicItem_777",   // Goose Portrait
    "BG30_MagicItem_801",   // Sinstone Sticker
    "BG30_MagicItem_828",   // Shaker Portrait
    "BG30_MagicItem_869",   // Felblood Portrait
    "BG30_MagicItem_879",   // Dalaran Cheese Wheel
    "BG30_MagicItem_900t",  // Dragonwing Glider
    "BG30_MagicItem_916",   // Essence of Dreams
    "BG30_MagicItem_919",   // Nazjatar Postcard
    "BG30_MagicItem_920",   // Spitescale Sushi Roll
    "BG30_MagicItem_923",   // Elementium Chest
    "BG30_MagicItem_930",   // Burgling Claw
    "BG30_MagicItem_942",   // Mecha-Jaraxxus Sticker
    "BG30_MagicItem_944",   // Redeemer Portrait
    "BG30_MagicItem_951",   // Lava Lamp
    "BG30_MagicItem_973",   // Minion Bait
    "BG30_MagicItem_979",   // Primordial Terrarium
    "BG30_MagicItem_981",   // The Eye of Dalaran
    "BG30_MagicItem_982",   // Shaman Prayer Beads
    "BG30_MagicItem_984",   // Charging Staff
    "BG30_MagicItem_984t",  // Charging Staff
    "BG30_MagicItem_986",   // Peacebloom Candle
    "BG30_MagicItem_987",   // Balladist Portrait
    "BG30_MagicItem_991",   // Felbat Portrait
    "BG30_MagicItem_992",   // Darnassus Pie
    "BG30_MagicItem_992t",  // Darnassus Pie
    "BG30_MagicItem_999",   // Fancy Spellbook
    "BG31_MagicItem_903",   // Wisdomball Supply
    "BG32_MagicItem_111",   // Toxic Stinger
    "BG32_MagicItem_170",   // Spell-powered Wrench
    "BG32_MagicItem_171",   // Divine Signet
    "BG32_MagicItem_204",   // Kel'Thuzad Portrait
    "BG32_MagicItem_230",   // Extravagant Scale
    "BG32_MagicItem_231",   // Gilded Anchor
    "BG32_MagicItem_231t",  // Gilded Anchor
    "BG32_MagicItem_270",   // Wicked Tome
    "BG32_MagicItem_276",   // Enigmatic Headstone
    "BG32_MagicItem_278",   // Reusable Batteries
    "BG32_MagicItem_279",   // Tough Tusk Sticker
    "BG32_MagicItem_281",   // Wizard's Pipe
    "BG32_MagicItem_282",   // Turbocharged Drill
    "BG32_MagicItem_283",   // Czarina Portrait
    "BG32_MagicItem_284",   // Aggem Sticker
    "BG32_MagicItem_286",   // Lavish Cape
    "BG32_MagicItem_304",   // Horn of Summoning
    "BG32_MagicItem_362",   // Innkeeper's Hearth
    "BG32_MagicItem_366",   // Guiding Candle
    "BG32_MagicItem_400",   // Shrine of Evolution
    "BG32_MagicItem_417",   // Tarecgosa Sticker
    "BG32_MagicItem_700",   // Marvelous Mushroom
    "BG32_MagicItem_801",   // Heart of the Forest
    "BG32_MagicItem_802",   // Fountain Pen
    "BG32_MagicItem_802t",  // Fountain Pen
    "BG32_MagicItem_804",   // Selfless Portrait
    "BG32_MagicItem_806",   // Battlecruiser Portrait
    "BG32_MagicItem_809",   // Gem Donation
    "BG32_MagicItem_823",   // Wax Imprinter
    "BG32_MagicItem_824",   // Implicator Portrait
    "BG32_MagicItem_830",   // Felemental Portrait
    "BG32_MagicItem_831",   // Sellemental Portrait
    "BG32_MagicItem_832",   // Windfall Portrait
    "BG32_MagicItem_832t",  // Windfall Portrait
    "BG32_MagicItem_844",   // Sacrificial Altar
    "BG32_MagicItem_887",   // Shadowy Elixir
    "BG32_MagicItem_888",   // Recycling Sticker
    "BG32_MagicItem_890",   // Cliffdiver Sticker
    "BG32_MagicItem_891",   // Finley's Helmet
    "BG32_MagicItem_893",   // Bluegill Flippers
    "BG32_MagicItem_894",   // Blessing Portrait
    "BG32_MagicItem_902",   // Statue of Hir'eek
    "BG32_MagicItem_906",   // Artanis Sticker
    "BG32_MagicItem_920",   // Thaumaturgist Portrait
    "BG32_MagicItem_922",   // Charming Panpipes
    "BG32_MagicItem_930",   // Archaic Scroll
    "BG32_MagicItem_931",   // Azsharan Statuette
    "BG32_MagicItem_933",   // Weary Portrait
    "BG32_MagicItem_935",   // Lubber Sticker
    "BG32_MagicItem_944",   // Empowerment Portrait
    "BG32_MagicItem_950",   // Gritty Portrait
    "BG32_MagicItem_953",   // Goldgrubber Portrait
    "BG32_MagicItem_954",   // Auric Offering
    "BG32_MagicItem_998",   // Behemoth Portrait
    "BG35_MagicItem_150",   // Cursed Crystal
    "BG35_MagicItem_151",   // Defiler Portrait
    "BG35_MagicItem_151t",  // Defiler Portrait
    "BG35_MagicItem_155",   // Felburned Ledger
    "BG35_MagicItem_300",   // Copper Coil
    "BG35_MagicItem_300t",  // Copper Coil
    "BG35_MagicItem_303",   // Skipper Portrait
    "BG35_MagicItem_309",   // Errgl Sticker
    "BG35_MagicItem_310",   // Radio Star Portrait
    "BG35_MagicItem_435",   // Bloodbound Ring
    "BG35_MagicItem_743",   // Electrode Attractor
    "BG35_MagicItem_803",   // Maxwell Sticker
    "BG35_MagicItem_803t",  // Maxwell Sticker
    "BG35_MagicItem_820",   // Safety Patch
    "BG35_MagicItem_821",   // Kaleidoscope
    "BG35_MagicItem_821t",  // Kaleidoscope
    "BG35_MagicItem_838",   // Double Stitch Needle
    "BG35_MagicItem_842",   // Egg of the Endtimes Portrait
    "BG35_MagicItem_848t",  // Egg of the Endtimes Portrait
    "BG35_MagicItem_851",   // Water Wheel
    "BG35_MagicItem_852",   // Lightning in a Bottle
    "BG35_MagicItem_861",   // Baller Portrait
    "BG35_MagicItem_863",   // Avalanche Sticker
    "BG35_MagicItem_870",   // Leapfrogger Portrait
    "BG35_MagicItem_920",   // Bubble Crown
    "BG35_MagicItem_921",   // Cowrie Necklace
    "BG35_MagicItem_922",   // Tide Raiser Portrait
    "BG35_MagicItem_924",   // Groundbreaker Portrait
    "BG35_MagicItem_925",   // Coral Spear
    "BG36_MagicItem_214",   // Insurrectionist's Blade
    "BG36_MagicItem_220",   // Glowing Crystal
    "BG36_MagicItem_305",   // Rune of Transmutation
    "BG36_MagicItem_307",   // Wand of Divination
    "BG36_MagicItem_362",   // Cyclist Portrait
    "BG36_MagicItem_371",   // Honeycomb Ring
    "BG36_MagicItem_380",   // Amplifying Essence
    "BG36_MagicItem_412",   // Kiri's Double Eclipse
    "BG36_MagicItem_412t2", // Kiri's Double Eclipse
    "BG36_MagicItem_417",   // Makeshift Master
    "BG36_MagicItem_602",   // Mask of Ancient Ones
    "BG36_MagicItem_606",   // Writhing Tentacles
    "BG36_MagicItem_801",   // Consuming Claw
    "BG36_MagicItem_810t",  // Inductive Gyroblade
    "BG36_MagicItem_812",   // Emergency Gearblade
    "BG36_MagicItem_840",   // Secret Schematic
    "BG36_MagicItem_850",   // Cookie's Stirring Rod
    "BGDUO_MagicItem_004",  // Charm of Generosity
    "BGDUO_MagicItem_005",  // Bottle Sticker
    "BGDUO_MagicItem_007",  // Orc-estra Portrait
    "BGDUO_MagicItem_009",  // Investment Contract
];

/// They act at the start of combat only and leave nothing a board taken after
/// it does not show (stats, keywords, minions with their own card rules):
/// not played, so they block the odds of a board taken before start of
/// combat.
pub const START_OF_COMBAT_POOL: &[&str] = &[
    // Hero powers.
    "BG22_HERO_000p",      // Deadeye
    "BG22_HERO_000p_t1",   // Aim Left!
    "BG22_HERO_000p_t2",   // Aim Low!
    "BG22_HERO_000p_t3",   // Aim High!
    "BG22_HERO_000p_t4",   // Aim Right!
    "TB_BaconShop_HP_019", // DIE, INSECTS!
    "TB_BaconShop_HP_027", // Fire the Cannons!
    "TB_BaconShop_HP_043", // Nefarious Fire
    "TB_BaconShop_HP_061", // ALL Will Burn!
    // Trinkets.
    "BG30_MagicItem_301",   // Eternal Portrait
    "BG30_MagicItem_310",   // Rivendare Portrait
    "BG30_MagicItem_403",   // Ironforge Anvil
    "BG30_MagicItem_441",   // Tinyfin Onesie
    "BG30_MagicItem_542",   // Emerald Dreamcatcher
    "BG30_MagicItem_821t2", // Fishy Sticker
    "BG30_MagicItem_918",   // Promo Portrait
    "BG30_MagicItem_970",   // Valorous Medallion
    "BG30_MagicItem_970t",  // Valorous Medallion
    "BG30_MagicItem_972",   // Karazhan Chess Set
    "BG32_MagicItem_280",   // Eclectic Shrine
    "BG32_MagicItem_360",   // Baleful Incense
    "BG32_MagicItem_365",   // Valdrakken Wind Chimes
    "BG32_MagicItem_419",   // Yu'lon Sticker
    "BG32_MagicItem_904",   // Hogwash Basin
    "BG32_MagicItem_932",   // Vash'jir Anemone
    "BG35_MagicItem_702",   // Stegodon Portrait
    "BG35_MagicItem_754",   // Dramaloc Sticker
    "BG35_MagicItem_834",   // Rylak Portrait
    "BG36_MagicItem_213",   // Lightfeather Sticker
    "BG36_MagicItem_361",   // Myrmidon Sticker
    "BGDUO_MagicItem_003",  // Summoning Sphere
];

/// They act during combat in a small, bounded way the engine does not play
/// (a Blood Gem value, a spell cast in combat, a Deathrattle that only gives
/// a card, a few stats on minions summoned in combat): approximate.
pub const IN_COMBAT_SMALL: &[&str] = &[
    // Trinkets.
    "BG30_MagicItem_410",   // Quilligraphy Set
    "BG30_MagicItem_410t2", // Quilligraphy Set
    "BG30_MagicItem_411",   // Hoggy Bank
    "BG30_MagicItem_422",   // Lorewalker Scroll
    "BG30_MagicItem_422t",  // Lorewalker Scroll
    "BG30_MagicItem_880",   // Feral Talisman
    "BG30_MagicItem_917",   // Rusty Trident
    "BG30_MagicItem_921",   // Flagbearer Portrait
    "BG30_MagicItem_988",   // Great Boar Sticker
    "BG30_MagicItem_989",   // Artisanal Urn
    "BG35_MagicItem_431t",  // Thornspike Pauldron
    "BG35_MagicItem_433",   // Vinespeaker Portrait
    "BG35_MagicItem_923",   // Bewitched Ribbon
];

/// They act during combat in a way the engine does not play and that is not
/// small (summons, lasting buffs, Deathrattles they give, minion types they
/// change): they block the odds by name, whenever the board was taken.
pub const IN_COMBAT_BLOCKS: &[&str] = &[
    // Hero powers.
    "BG23_HERO_201p",      // Tentacular
    "TB_BaconShop_HP_017", // Murloc King
    // Trinkets.
    "BG30_MagicItem_303",   // Automaton Portrait
    "BG30_MagicItem_407",   // Ship in a Bottle
    "BG30_MagicItem_427",   // Tiger Carving
    "BG30_MagicItem_427t",  // Tiger Carving
    "BG30_MagicItem_432",   // Belcher Portrait
    "BG30_MagicItem_432t",  // Belcher Portrait
    "BG30_MagicItem_433",   // Alliance Keychain
    "BG30_MagicItem_433t",  // Alliance Keychain
    "BG30_MagicItem_438t",  // Mug of the Sire
    "BG30_MagicItem_440",   // Boom Controller
    "BG30_MagicItem_540",   // Slamma Sticker
    "BG30_MagicItem_803",   // Kaboom Bot Portrait
    "BG30_MagicItem_822",   // Twin Sky Lanterns
    "BG30_MagicItem_822t2", // Twin Sky Lanterns
    "BG30_MagicItem_825",   // Smuggler Portrait
    "BG30_MagicItem_843t",  // Horde Keychain
    "BG30_MagicItem_864",   // Gilnean Thorned Rose
    "BG30_MagicItem_880t",  // Feral Talisman
    "BG30_MagicItem_886",   // Reinforced Shield
    "BG30_MagicItem_910",   // Mechagon Adapter
    "BG30_MagicItem_925",   // Ceremonial Sword
    "BG30_MagicItem_952",   // Jarred Frostling
    "BG30_MagicItem_971",   // Enforcer Portrait
    "BG30_MagicItem_978",   // Blingtron's Sunglasses
    "BG30_MagicItem_989t",  // Artisanal Urn
    "BG32_MagicItem_179",   // Drakkari Portrait
    "BG32_MagicItem_274",   // Bristlebach Portrait
    "BG32_MagicItem_301",   // Bassgill Portrait
    "BG32_MagicItem_415",   // Battle Horn
    "BG32_MagicItem_803",   // Macaw Portrait
    "BG32_MagicItem_820",   // Impulsive Portrait
    "BG32_MagicItem_862",   // Unholy Sanctum
    "BG32_MagicItem_862t",  // Unholy Sanctum
    "BG32_MagicItem_907",   // S'Thara Sticker
    "BG32_MagicItem_934",   // Dazzling Dagger
    "BG32_MagicItem_960",   // Crocheted Sungill
    "BG35_MagicItem_430",   // Scrapsmith Portrait
    "BG35_MagicItem_432",   // Blood Amulet
    "BG35_MagicItem_701",   // Fang Anklet
    "BG35_MagicItem_714",   // Powder Keg
    "BG35_MagicItem_731",   // Deathtouch Apple
    "BG35_MagicItem_740",   // Sky Golem Portrait
    "BG35_MagicItem_849",   // Cloud Serpent Horn
    "BG35_MagicItem_871",   // Mama Bear Sticker
    "BG36_MagicItem_205",   // Deathwhisper Sticker
    "BG36_MagicItem_403",   // Hammer of Twilight
    "BG36_MagicItem_403t",  // Hammer of Twilight
    "BG36_MagicItem_841",   // Assembler Portrait
];
