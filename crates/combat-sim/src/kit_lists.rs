//! The hero powers and trinkets the engine has reviewed (T-628), one list per
//! kind of effect ([`crate::kit::KitEffect`]): every power and trinket seen
//! on a board that fought in the user's games (logs and saved history of
//! 2026-10-18, build 254477), checked one by one against its text in
//! HearthstoneJSON. Names only, as comments; no card text.

/// Their text never changes a combat: the shop, the hand, the Tavern, a
/// minion they give (played by its own card rules), a keyword the board
/// already shows, or a counter that only pays out in the shop. The two
/// placeholders of a trinket slot not bought yet are here too.
pub const NO_COMBAT_EFFECT: &[&str] = &[
    // Hero powers.
    "BG20_HERO_101p",       // See the Light
    "BG20_HERO_102p",       // For the Horde!
    "BG20_HERO_103p",       // Bloodbound
    "BG20_HERO_202p",       // Power of the Storm
    "BG20_HERO_242p",       // Natural Balance
    "BG20_HERO_280p5",      // Glaive Ricochet
    "BG20_HERO_283p",       // Dungar's Gryphon
    "BG20_HERO_301p",       // Devour
    "BG21_HERO_000p",       // Conviction
    "BG21_HERO_010p",       // I Spy
    "BG21_HERO_020p",       // Stir the Pot
    "BG21_HERO_030p",       // Pilot the Shredder
    "BG22_HERO_004p",       // Twice as Nice
    "BG22_HERO_200p",       // MechGyver
    "BG22_HERO_201p",       // Expedition Plans
    "BG23_HERO_306p",       // Reclaimed Souls
    "BG24_HERO_100p",       // Whodunit?
    "BG24_HERO_204p",       // Enhancification
    "BG25_HERO_100p",       // Build-An-Undead
    "BG25_HERO_105p",       // Sign a New Artist
    "BG26_HERO_101p",       // I'm the Cap'n Now
    "BG26_HERO_102p",       // Major Hymn
    "BG26_HERO_102p2",      // Minor Hymn
    "BG26_HERO_104p",       // Upbeat Harmony
    "BG27_HERO_801p2",      // Choose Your Champion
    "BG28_HERO_400p",       // Lucky Roll
    "BG28_HERO_400p2",      // Lucky Roll
    "BG28_HERO_800p",       // Reliquary Research
    "BG28_HERO_801p",       // Blessing of the Nine Frogs
    "BG30_HERO_304p",       // Fantastic Treasure
    "BG31_HERO_003p",       // The Galaxy's Lens
    "BG31_HERO_005p",       // Cloning Gallery
    "BG31_HERO_006p",       // Arcane Knowledge
    "BG31_HERO_801p",       // Lift Off
    "BG31_HERO_802p",       // Warp Gate
    "BG31_HERO_811p",       // Spawning Pool
    "BG31_HERO_811p2",      // Evolution Chamber
    "BG32_HERO_001p",       // Wisdom of Ancients
    "BG32_HERO_002p",       // Growing Collection
    "BG33_HERO_001p_ALT",   // Heroic Inspiration
    "BG34_HERO_000p",       // Alternate Timeline
    "BG34_HERO_001p",       // Mana Per Minute
    "BG34_HERO_004p",       // Warped Conflux
    "BG35_HERO_001p",       // King of Duality
    "BG36_HERO_000p",       // Incubate
    "BG36_HERO_002p",       // Dark Ritual
    "BG36_HERO_101p",       // Void Power
    "BG36_HERO_105p",       // Feel Devastation
    "BGDUO_HERO_100p",      // [x]
    "BGDUO_HERO_104p",      // Efficient Exchange
    "TB_BaconShop_HP_001",  // Sharpen Blades
    "TB_BaconShop_HP_008",  // Smart Savings
    "TB_BaconShop_HP_010",  // Boon of Light
    "TB_BaconShop_HP_011",  // Galakrond's Greed
    "TB_BaconShop_HP_014",  // Stay Frosty
    "TB_BaconShop_HP_015",  // Tinker
    "TB_BaconShop_HP_022",  // Snicker-snack
    "TB_BaconShop_HP_024",  // Reborn Rites
    "TB_BaconShop_HP_028",  // Temporal Tavern
    "TB_BaconShop_HP_033",  // Menagerist
    "TB_BaconShop_HP_035",  // All Patched Up
    "TB_BaconShop_HP_036",  // Bloodfury
    "TB_BaconShop_HP_038",  // Bananarama
    "TB_BaconShop_HP_039t", // Puzzle Box
    "TB_BaconShop_HP_040",  // Brick by Brick
    "TB_BaconShop_HP_041",  // A Tale of Kings
    "TB_BaconShop_HP_041a", // King of Beasts
    "TB_BaconShop_HP_041b", // King of Mechs
    "TB_BaconShop_HP_041c", // King of Murlocs
    "TB_BaconShop_HP_041d", // King of Demons
    "TB_BaconShop_HP_041f", // King of Dragons
    "TB_BaconShop_HP_041g", // King of Pirates
    "TB_BaconShop_HP_041h", // King of Elementals
    "TB_BaconShop_HP_041i", // King of Quilboar
    "TB_BaconShop_HP_041j", // King of Naga
    "TB_BaconShop_HP_041k", // King of Undead
    "TB_BaconShop_HP_041l", // King of Aberrations
    "TB_BaconShop_HP_042",  // Hat Trick
    "TB_BaconShop_HP_044",  // Procrastinate
    "TB_BaconShop_HP_046",  // Gonna Be Rich!
    "TB_BaconShop_HP_047",  // Lead Explorer
    "TB_BaconShop_HP_048",  // Battle Brand
    "TB_BaconShop_HP_049",  // Graveyard Shift
    "TB_BaconShop_HP_052",  // Arcane Alteration
    "TB_BaconShop_HP_053",  // I'll Take That!
    "TB_BaconShop_HP_054",  // Manastorm
    "TB_BaconShop_HP_056",  // Gone Fishing
    "TB_BaconShop_HP_063",  // Clairvoyance
    "TB_BaconShop_HP_064",  // Queen of Dragons
    "TB_BaconShop_HP_066",  // Verdant Spheres
    "TB_BaconShop_HP_072",  // Pirate Parrrrty!
    "TB_BaconShop_HP_074",  // Buried Treasure
    "TB_BaconShop_HP_075",  // Trash for Treasure
    "TB_BaconShop_HP_076",  // Piggy Bank
    "TB_BaconShop_HP_077",  // Bob's Burgles
    "TB_BaconShop_HP_080",  // Kel'Thuzad's Kitty
    "TB_BaconShop_HP_081",  // Friendly Wager
    "TB_BaconShop_HP_082",  // Everbloom
    "TB_BaconShop_HP_084",  // Swap, Lock, & Shop It
    "TB_BaconShop_HP_085t", // Tavern Lighting
    "TB_BaconShop_HP_101",  // Come One, Come All!
    "TB_BaconShop_HP_102",  // Three Wishes
    "TB_BaconShop_HP_104",  // Saturday C'Thuns!
    "TB_BaconShop_HP_105",  // Avatar of N'Zoth
    "TB_BaconShop_HP_106",  // Prize Wall
    "TB_BaconShop_HP_702t", // Rune of Damnation
    // Trinkets.
    "BG30_MagicItem_406",   // Butcher's Sickle
    "BG30_MagicItem_416",   // Token of the Old Gods
    "BG30_MagicItem_420",   // Book of Medivh
    "BG30_MagicItem_420t",  // Book of Medivh
    "BG30_MagicItem_423",   // Innkeeper's Stein
    "BG30_MagicItem_425",   // Azeroth Model Globe
    "BG30_MagicItem_426",   // Colorful Compass
    "BG30_MagicItem_426t",  // Colorful Compass
    "BG30_MagicItem_430",   // Rockin' Music Box
    "BG30_MagicItem_431",   // Azerite Portrait
    "BG30_MagicItem_435",   // Goldenizer Supply
    "BG30_MagicItem_439",   // Designer Eyepatch
    "BG30_MagicItem_541",   // Nether Pendant
    "BG30_MagicItem_544t",  // Nomi Sticker
    "BG30_MagicItem_547",   // Comfy Coffin
    "BG30_MagicItem_547t",  // Comfy Coffin
    "BG30_MagicItem_555",   // Surprise Portrait
    "BG30_MagicItem_703",   // Mystery Cube
    "BG30_MagicItem_705",   // Bartend-o-Tron's Oilcan
    "BG30_MagicItem_706",   // Reflective Pendant
    "BG30_MagicItem_707",   // Tickatus Sticker
    "BG30_MagicItem_713",   // Bleeding Heart
    "BG30_MagicItem_821",   // Fish Portrait
    "BG30_MagicItem_841",   // Glowing Gauntlet
    "BG30_MagicItem_847",   // Goblin Wallet
    "BG30_MagicItem_868",   // Rewinder Portrait
    "BG30_MagicItem_876",   // Manipulator Portrait
    "BG30_MagicItem_879t",  // Dalaran Cheese Wheel
    "BG30_MagicItem_888",   // Souvenir Stand
    "BG30_MagicItem_891",   // Trip Vouchers
    "BG30_MagicItem_900",   // Dragonwing Glider
    "BG30_MagicItem_914",   // Nerglish Phrasebook
    "BG30_MagicItem_914t",  // Nerglish Phrasebook
    "BG30_MagicItem_924",   // Booty Bay Brew
    "BG30_MagicItem_924t",  // Booty Bay Brew
    "BG30_MagicItem_931",   // Lucky Tabby
    "BG30_MagicItem_943",   // Surveyor Portrait
    "BG30_MagicItem_993",   // Pagle's Fishing Rod
    "BG30_MagicItem_994",   // Yogg-Tastic Pastry
    "BG30_MagicItem_996",   // Bob's Tip Jar
    "BG30_MagicItem_998",   // Bob-blehead
    "BG30_Trinket_1st",     // Lesser Trinket
    "BG30_Trinket_2nd",     // Greater Trinket
    "BG32_MagicItem_172",   // Boom's Monster Portrait
    "BG32_MagicItem_205",   // Maw Caster Portrait
    "BG32_MagicItem_232",   // Shark Cannon
    "BG32_MagicItem_270t",  // Wicked Tome
    "BG32_MagicItem_271",   // Ornate Clock
    "BG32_MagicItem_300",   // Putricide Sticker
    "BG32_MagicItem_350",   // Splinter of Aurum
    "BG32_MagicItem_361",   // Portable Factory
    "BG32_MagicItem_361t",  // Portable Factory
    "BG32_MagicItem_362t",  // Innkeeper's Hearth
    "BG32_MagicItem_364",   // Poet Portrait
    "BG32_MagicItem_367",   // Ghastly Sticker
    "BG32_MagicItem_416",   // War Drum
    "BG32_MagicItem_428",   // Worn Treasure Map
    "BG32_MagicItem_801t",  // Heart of the Forest
    "BG32_MagicItem_807",   // Curator Sticker
    "BG32_MagicItem_808",   // Bloodbound Earrings
    "BG32_MagicItem_808t",  // Bloodbound Earrings
    "BG32_MagicItem_817",   // Rendle Sticker
    "BG32_MagicItem_821",   // Pilgrimp Sticker
    "BG32_MagicItem_822",   // Bazaar Sticker
    "BG32_MagicItem_858",   // Explorer's Binoculars
    "BG32_MagicItem_892",   // Vibrant Bubble
    "BG32_MagicItem_901",   // Gold-plated Compass
    "BG32_MagicItem_925",   // Hackerfin Portrait
    "BG32_MagicItem_926",   // Morgl Portrait
    "BG32_MagicItem_951",   // Gold Pendant
    "BG32_MagicItem_957",   // Grifter Portrait
    "BG35_MagicItem_152",   // Demonic Tapestry
    "BG35_MagicItem_154",   // Ur'zul Sticker
    "BG35_MagicItem_156",   // Flaming Portrait
    "BG35_MagicItem_301",   // Scraper Sticker
    "BG35_MagicItem_302",   // Stormcoil Sticker
    "BG35_MagicItem_305",   // Conch Portrait
    "BG35_MagicItem_306",   // Jailer Sticker
    "BG35_MagicItem_434",   // Jewelry Box
    "BG35_MagicItem_700",   // Wildfeather Duster
    "BG35_MagicItem_710",   // Miniature Ship
    "BG35_MagicItem_712",   // Privateer Portrait
    "BG35_MagicItem_713",   // Trusty Crowbar
    "BG35_MagicItem_733",   // Jailer Sticker
    "BG35_MagicItem_741",   // Beatboxer Portrait
    "BG35_MagicItem_742",   // Accord-o-Tron Portrait
    "BG35_MagicItem_750",   // Magicfin Sticker
    "BG35_MagicItem_752",   // Young Murk-Eye Sticker
    "BG35_MagicItem_753",   // Murky Sticker
    "BG35_MagicItem_755",   // Chillmere Mosaic
    "BG35_MagicItem_801",   // Sous Chef Sticker
    "BG35_MagicItem_812",   // Corrupted Tome
    "BG35_MagicItem_814",   // Stuffed Coin Purse
    "BG35_MagicItem_815",   // Magician's Top Hat
    "BG35_MagicItem_816",   // Orb of the Unknown
    "BG35_MagicItem_816t",  // Orb of the Unknown
    "BG35_MagicItem_817",   // Lens Case
    "BG35_MagicItem_818",   // Mysterious Orb
    "BG35_MagicItem_823",   // Timeworn Candelabra
    "BG35_MagicItem_823t",  // Timeworn Candelabra
    "BG35_MagicItem_840",   // Chromatic Tear
    "BG35_MagicItem_840t",  // Chromatic Tear
    "BG35_MagicItem_850",   // Pocket Cyclone
    "BG35_MagicItem_850t",  // Pocket Cyclone
    "BG35_MagicItem_862",   // Upstart Embers
    "BG35_MagicItem_872",   // Ophidian Staff
    "BG35_MagicItem_890",   // Sunken Anchor
    "BG35_MagicItem_930",   // Warband Whistle
    "BG35_MagicItem_931",   // Transcribing Typewriter
    "BG35_MagicItem_931t",  // Transcribing Typewriter
    "BG36_MagicItem_200",   // Herding Horn
    "BG36_MagicItem_202",   // Warcry Totem
    "BG36_MagicItem_203",   // Dragon Skull
    "BG36_MagicItem_204",   // Plaguerunner Portrait
    "BG36_MagicItem_206",   // Ominous Stone
    "BG36_MagicItem_208",   // Floating Candle Set
    "BG36_MagicItem_211",   // Lovely Locket
    "BG36_MagicItem_212",   // Wolfhead Flail
    "BG36_MagicItem_215",   // Dragon's Eye
    "BG36_MagicItem_217",   // Funeral Wreath
    "BG36_MagicItem_300",   // Frigid Blossom
    "BG36_MagicItem_301",   // Lockbox Portrait
    "BG36_MagicItem_302",   // Gold Mallet
    "BG36_MagicItem_302t",  // Gold Mallet
    "BG36_MagicItem_303",   // Glass of Perspective
    "BG36_MagicItem_303t",  // Glass of Perspective
    "BG36_MagicItem_308",   // Trailblazer Sticker
    "BG36_MagicItem_309",   // Wax Lance
    "BG36_MagicItem_363",   // Escapee Portrait
    "BG36_MagicItem_370",   // Maldraxxus Dagger
    "BG36_MagicItem_372",   // Sphere of Memory
    "BG36_MagicItem_373",   // Multilayered Shield
    "BG36_MagicItem_390",   // Baller Portrait
    "BG36_MagicItem_400",   // Corrupted Coin Portrait
    "BG36_MagicItem_402",   // Mindslasher Portrait
    "BG36_MagicItem_404",   // Corrupted Baton
    "BG36_MagicItem_404t",  // Corrupted Baton
    "BG36_MagicItem_406",   // Shath'Yar Shrine
    "BG36_MagicItem_407",   // Dragonhead Staff
    "BG36_MagicItem_408",   // Quartermaster's Hook
    "BG36_MagicItem_409",   // Ruby Tusk
    "BG36_MagicItem_410",   // Soul Battery
    "BG36_MagicItem_411",   // Reinvigorating Light
    "BG36_MagicItem_414",   // Weighted Gauntlet
    "BG36_MagicItem_418",   // Tome of the Ancients
    "BG36_MagicItem_423",   // Volumizer Portrait
    "BG36_MagicItem_423t",  // Volumizer Portrait
    "BG36_MagicItem_424",   // Rascal Sticker
    "BG36_MagicItem_430",   // Sludge Portrait
    "BG36_MagicItem_609",   // Trickster's Sleeve
    "BG36_MagicItem_610",   // Sinister Invitation
    "BG36_MagicItem_611",   // Tableware Portrait
    "BG36_MagicItem_800",   // Demonic Bloodletter
    "BG36_MagicItem_810",   // Inductive Gyroblade
    "BG36_MagicItem_811",   // Defensive Shell
    "BG36_MagicItem_820",   // Flighty Portrait
    "BG36_MagicItem_830",   // Bloodfury Shield
    "BG36_MagicItem_831",   // Felsteel Cleaver
    "BG36_MagicItem_852",   // Willbreaker Sticker
    "BGDUO_MagicItem_001",  // Common Thread
    "BGDUO_MagicItem_002",  // Cho'gall Sticker
    "BGDUO_MagicItem_006",  // Magic Road Sign
    "BGDUO_MagicItem_008",  // Moveout Box
    "BGDUO_MagicItem_010",  // Empowered Mirror
    "BGDUO_MagicItem_010t", // Empowered Mirror
];

/// They act at the start of combat only.
pub const START_OF_COMBAT: &[&str] = &[
    // Hero powers.
    "BG20_HERO_282p",       // Fragrant Phylactery
    "BG22_HERO_001p",       // Embrace the Elements
    "BG22_HERO_001p_t1",    // Earth Invocation
    "BG22_HERO_001p_t2",    // Fire Invocation
    "BG22_HERO_001p_t3",    // Water Invocation
    "BG22_HERO_001p_t4",    // Lightning Invocation
    "BGDUO_HERO_101p",      // Glorious Gloop
    "TB_BaconShop_HP_037a", // Wax Warband
    "TB_BaconShop_HP_069",  // Wingmen
    "TB_BaconShop_HP_086",  // Swatting Insects
    "TB_BaconShop_HP_103",  // Embrace Your Rage
    // Trinkets.
    "BG30_MagicItem_902", // Holy Mallet
    "BG30_MagicItem_962", // Training Certificate
    "BG30_MagicItem_995", // Bronze Timepiece
    "BG32_MagicItem_306", // Herald Sticker
    "BG35_MagicItem_711", // Protective Ring
    "BG36_MagicItem_600", // Converter Portrait
];

/// They act at the start of combat and may act again later in it ("when
/// you have space", "after your last minion dies").
pub const START_AND_IN_COMBAT: &[&str] = &[
    // Hero powers.
    "BG22_HERO_000p_Alt", // Lock and Load
    "BG22_HERO_002p",     // Frostwolf Fervor
    "BG22_HERO_003p",     // Stormpike Strength
    "BG25_HERO_103p",     // Rapid Reanimation
    // Trinkets.
    "BG35_MagicItem_732", // Soul Fermenter
];

/// They act during the combat, after its start (or change how a minion or
/// Blood Gem acts in it).
pub const IN_COMBAT: &[&str] = &[
    // Hero powers.
    "BG20_HERO_100p",      // Glory of Combat
    "BG22_HERO_305p",      // Broodmother
    "TB_BaconShop_HP_020", // Prestidigitation
    "TB_BaconShop_HP_107", // Sprout It Out!
    // Trinkets.
    "BG30_MagicItem_418",  // Bronzebeard Portrait
    "BG30_MagicItem_437",  // Staff of the Scourge
    "BG30_MagicItem_442",  // Blood Golem Sticker
    "BG30_MagicItem_546",  // Jar o' Gems
    "BG30_MagicItem_700",  // Deathly Phylactery
    "BG30_MagicItem_988t", // Great Boar Sticker
    "BG32_MagicItem_200",  // All-Purpose Kibble
    "BG32_MagicItem_363",  // Faerie Dragon Scale
    "BG32_MagicItem_860t", // Beetle Band
    "BG36_MagicItem_201",  // Lionfish Portrait
    "BG36_MagicItem_216",  // Eternal Portrait
    "BG36_MagicItem_416",  // Evil Experiment
    "BG36_MagicItem_450",  // Snarky Portrait
];

/// Ancient Wishbone: the hero power triggers twice.
pub const TWICE_HERO_POWER: &[&str] = &[
    // Trinkets.
    "BG30_MagicItem_804", // Ancient Wishbone
];
