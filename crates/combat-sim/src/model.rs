//! What goes into a combat: two warbands of up to seven minions each.
//!
//! A minion is what the board showed as the combat began: its current attack,
//! health and keywords. Card facts the board does not show (tavern tier,
//! tribes, the card's own stats and keywords for Reborn) come from the card
//! data; `None` is unknown, never 0.

use std::fmt;

/// Most minions a warband can hold.
pub const MAX_BOARD: usize = 7;

/// Keywords as a small bit set.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Keywords(u16);

impl Keywords {
    pub const NONE: Keywords = Keywords(0);
    pub const TAUNT: Keywords = Keywords(1);
    pub const DIVINE_SHIELD: Keywords = Keywords(1 << 1);
    pub const REBORN: Keywords = Keywords(1 << 2);
    pub const VENOMOUS: Keywords = Keywords(1 << 3);
    pub const POISONOUS: Keywords = Keywords(1 << 4);
    pub const WINDFURY: Keywords = Keywords(1 << 5);
    pub const MEGA_WINDFURY: Keywords = Keywords(1 << 6);
    pub const STEALTH: Keywords = Keywords(1 << 7);

    /// Keyword names as the parser saves them (`bg_parser::report::KEYWORDS`)
    /// and as HearthstoneJSON spells them, mapped to the bit.
    const NAMES: [(&'static str, &'static str, Keywords); 8] = [
        ("taunt", "TAUNT", Keywords::TAUNT),
        ("divine_shield", "DIVINE_SHIELD", Keywords::DIVINE_SHIELD),
        ("reborn", "REBORN", Keywords::REBORN),
        ("venomous", "VENOMOUS", Keywords::VENOMOUS),
        ("poisonous", "POISONOUS", Keywords::POISONOUS),
        ("windfury", "WINDFURY", Keywords::WINDFURY),
        ("mega_windfury", "MEGA_WINDFURY", Keywords::MEGA_WINDFURY),
        ("stealth", "STEALTH", Keywords::STEALTH),
    ];

    pub const fn union(self, other: Keywords) -> Keywords {
        Keywords(self.0 | other.0)
    }

    pub const fn contains(self, other: Keywords) -> bool {
        self.0 & other.0 == other.0 && other.0 != 0
    }

    pub const fn without(self, other: Keywords) -> Keywords {
        Keywords(self.0 & !other.0)
    }

    pub const fn intersection(self, other: Keywords) -> Keywords {
        Keywords(self.0 & other.0)
    }

    /// A keyword by its saved name (`divine_shield`) or its card data tag
    /// (`DIVINE_SHIELD`). Other names (deathrattle, rally) are card text,
    /// not keywords the engine tracks: `None`.
    pub fn from_name(name: &str) -> Option<Keywords> {
        Keywords::NAMES
            .iter()
            .find(|(saved, tag, _)| *saved == name || *tag == name)
            .map(|(_, _, k)| *k)
    }
}

/// Minion types as a bit set ("tribes"). `ALL` is the type that counts as
/// every type.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Tribes(u16);

impl Tribes {
    pub const NONE: Tribes = Tribes(0);
    pub const BEAST: Tribes = Tribes(1);
    pub const DEMON: Tribes = Tribes(1 << 1);
    pub const DRAGON: Tribes = Tribes(1 << 2);
    pub const ELEMENTAL: Tribes = Tribes(1 << 3);
    pub const MECH: Tribes = Tribes(1 << 4);
    pub const MURLOC: Tribes = Tribes(1 << 5);
    pub const NAGA: Tribes = Tribes(1 << 6);
    pub const PIRATE: Tribes = Tribes(1 << 7);
    pub const QUILBOAR: Tribes = Tribes(1 << 8);
    pub const UNDEAD: Tribes = Tribes(1 << 9);
    pub const ABERRATION: Tribes = Tribes(1 << 10);
    pub const ALL: Tribes = Tribes(1 << 15);

    /// HearthstoneJSON race names (`MECHANICAL`, `ALL`...).
    const NAMES: [(&'static str, Tribes); 12] = [
        ("BEAST", Tribes::BEAST),
        ("DEMON", Tribes::DEMON),
        ("DRAGON", Tribes::DRAGON),
        ("ELEMENTAL", Tribes::ELEMENTAL),
        ("MECHANICAL", Tribes::MECH),
        ("MURLOC", Tribes::MURLOC),
        ("NAGA", Tribes::NAGA),
        ("PIRATE", Tribes::PIRATE),
        ("QUILBOAR", Tribes::QUILBOAR),
        ("UNDEAD", Tribes::UNDEAD),
        ("ABERRATION", Tribes::ABERRATION),
        ("ALL", Tribes::ALL),
    ];

    pub const fn union(self, other: Tribes) -> Tribes {
        Tribes(self.0 | other.0)
    }

    /// True when this minion counts as `tribe` (an "all types" minion counts
    /// as every one).
    pub const fn is(self, tribe: Tribes) -> bool {
        self.0 & tribe.0 != 0 || (self.0 & Tribes::ALL.0 != 0 && tribe.0 != 0)
    }

    pub fn from_name(name: &str) -> Option<Tribes> {
        Tribes::NAMES
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, t)| *t)
    }
}

/// The card's own stats and keywords, for a minion that comes back with
/// Reborn (it returns as a fresh copy of the card with 1 Health).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BaseCard {
    pub attack: i32,
    pub health: i32,
    pub keywords: Keywords,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Minion {
    /// The exact card id on the board; golden cards have their own id
    /// (`BG25_354_G`, `TB_BaconUps_045`), and the card rules use it.
    pub card_id: String,
    pub golden: bool,
    pub attack: i32,
    pub health: i32,
    pub keywords: Keywords,
    pub tribes: Tribes,
    /// Tavern tier, for the damage a surviving minion deals.
    pub tier: Option<u8>,
    pub base: Option<BaseCard>,
    /// The Dark Gifts the minion carries, by the gift's card id
    /// (`BG36_MidGameEffect_000t16`), as the offer names it (D-058); see
    /// [`crate::gifts`]. Empty for most minions.
    pub gifts: Vec<String>,
    /// The two numbers its text shows, for a card whose start-of-combat buff
    /// grows during the game (Hopebringer, Ultraviolet Ascendant; T-628);
    /// 0 where not given.
    pub counters: [i32; 2],
}

impl Minion {
    /// A minion with no keywords, no tribe and unknown card facts. Tests and
    /// callers fill in the rest.
    pub fn new(card_id: impl Into<String>, attack: i32, health: i32) -> Self {
        Minion {
            card_id: card_id.into(),
            golden: false,
            attack,
            health,
            keywords: Keywords::NONE,
            tribes: Tribes::NONE,
            tier: None,
            base: None,
            gifts: Vec::new(),
            counters: [0, 0],
        }
    }

    pub fn with(mut self, keywords: Keywords) -> Self {
        self.keywords = self.keywords.union(keywords);
        self
    }

    pub fn tribe(mut self, tribes: Tribes) -> Self {
        self.tribes = self.tribes.union(tribes);
        self
    }

    pub fn tier(mut self, tier: u8) -> Self {
        self.tier = Some(tier);
        self
    }

    /// Adds a Dark Gift by its card id.
    pub fn gift(mut self, id: impl Into<String>) -> Self {
        self.gifts.push(id.into());
        self
    }
}

/// Game-long values of a warband that change what its cards do in combat
/// (T-609). Zero when unknown: the cards' printed values are used.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GameValues {
    /// What each Blood Gem gives beyond +1/+1 (Attack, Health).
    pub blood_gem: (i32, i32),
    /// What each buff of a Tavern spell gives beyond its text.
    pub spell: (i32, i32),
    /// Eternal Knights that died this game: each Knight has +4/+2 for each
    /// (golden +8/+4).
    pub knights_dead: i32,
    /// Attack the side's Undead have on top this game.
    pub undead_attack: i32,
    /// Attack and Health the side's Beetles have on top this game.
    pub beetle: (i32, i32),
}

/// A hero power or trinket of a board's hero (T-628), by its card id.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KitCard {
    pub card_id: String,
    /// Hero powers: 0 the hero's own, 1 a second one. Trinkets: 1 lesser,
    /// 2 greater, 3 a hero power that became a trinket.
    pub slot: u8,
    /// The numbers its text shows, in order; 0 where the game gives none.
    pub counters: [i32; 5],
    /// The friendly minion it names, by its index on the board (0 is the
    /// left-most).
    pub target: Option<usize>,
    /// The minion it holds off the board (Lock and Load); `None` when it
    /// holds none or it is not shown.
    pub held: Option<Minion>,
}

impl KitCard {
    pub fn new(card_id: impl Into<String>) -> Self {
        KitCard {
            card_id: card_id.into(),
            ..KitCard::default()
        }
    }
}

/// A board's hero powers and trinkets (T-628). See [`crate::kit`] for what
/// the engine knows of them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Kit {
    pub powers: Vec<KitCard>,
    pub trinkets: Vec<KitCard>,
    /// The game's round as the combat starts, for the hero powers that
    /// unlock on a turn ("Unlocks on Turn 7"); `None` when not given.
    pub round: Option<u32>,
}

impl Kit {
    /// Every hero power, then every trinket.
    pub fn cards(&self) -> impl Iterator<Item = &KitCard> {
        self.powers.iter().chain(&self.trinkets)
    }
}

/// One side of a combat: its minions left to right, its hero's tavern
/// tier (part of the damage when it wins; `None` when unknown), its
/// game-long values and its hero's powers and trinkets (`None` when the
/// caller does not know them: then they are not checked, as before T-628;
/// an empty [`Kit`] is a hero with none).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Warband {
    pub minions: Vec<Minion>,
    pub tavern_tier: Option<u8>,
    pub values: GameValues,
    pub kit: Option<Kit>,
}

impl Warband {
    pub fn new(minions: Vec<Minion>) -> Self {
        Warband {
            minions,
            tavern_tier: None,
            values: GameValues::default(),
            kit: None,
        }
    }

    pub fn validate(&self) -> Result<(), BoardError> {
        if self.minions.len() > MAX_BOARD {
            return Err(BoardError::TooManyMinions(self.minions.len()));
        }
        for (i, m) in self.minions.iter().enumerate() {
            if m.health <= 0 {
                return Err(BoardError::NoHealth(i));
            }
            if m.attack < 0 {
                return Err(BoardError::NegativeAttack(i));
            }
        }
        Ok(())
    }
}

/// One side of a combat. In Duos a team brings two boards: the one that
/// fights first and the teammate's, which comes in once the first is gone
/// (`second` is `None` in Solo, or when the teammate is out of the game).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Team {
    pub first: Warband,
    pub second: Option<Warband>,
}

impl Team {
    pub fn solo(board: Warband) -> Self {
        Team {
            first: board,
            second: None,
        }
    }

    pub fn duo(first: Warband, second: Warband) -> Self {
        Team {
            first,
            second: Some(second),
        }
    }

    /// Every board of the team, first one first.
    pub fn boards(&self) -> impl Iterator<Item = &Warband> {
        std::iter::once(&self.first).chain(self.second.as_ref())
    }

    pub fn validate(&self) -> Result<(), BoardError> {
        self.boards().try_for_each(Warband::validate)
    }
}

/// A board the engine refuses: the input is wrong, not the combat.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoardError {
    TooManyMinions(usize),
    NoHealth(usize),
    NegativeAttack(usize),
    NoTrials,
}

impl fmt::Display for BoardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BoardError::TooManyMinions(n) => {
                write!(f, "a warband holds at most {MAX_BOARD} minions, got {n}")
            }
            BoardError::NoHealth(i) => write!(f, "minion {i} has no health left"),
            BoardError::NegativeAttack(i) => write!(f, "minion {i} has negative attack"),
            BoardError::NoTrials => write!(f, "at least one trial is needed"),
        }
    }
}

impl std::error::Error for BoardError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keywords_by_saved_name_or_card_tag() {
        assert_eq!(
            Keywords::from_name("divine_shield"),
            Some(Keywords::DIVINE_SHIELD)
        );
        assert_eq!(Keywords::from_name("WINDFURY"), Some(Keywords::WINDFURY));
        assert_eq!(Keywords::from_name("deathrattle"), None);
        let k = Keywords::TAUNT.union(Keywords::REBORN);
        assert!(k.contains(Keywords::TAUNT));
        assert!(!k.without(Keywords::TAUNT).contains(Keywords::TAUNT));
        assert!(!Keywords::NONE.contains(Keywords::NONE));
    }

    #[test]
    fn an_all_types_minion_counts_as_every_tribe() {
        assert!(Tribes::ALL.is(Tribes::BEAST));
        assert!(Tribes::from_name("MECHANICAL").unwrap().is(Tribes::MECH));
        assert!(!Tribes::DRAGON.is(Tribes::BEAST));
        assert!(!Tribes::ALL.is(Tribes::NONE));
    }

    #[test]
    fn rejects_impossible_boards() {
        let full = Warband::new(vec![Minion::new("X", 1, 1); 8]);
        assert_eq!(full.validate(), Err(BoardError::TooManyMinions(8)));
        let dead = Warband::new(vec![Minion::new("X", 1, 0)]);
        assert_eq!(dead.validate(), Err(BoardError::NoHealth(0)));
        let neg = Warband::new(vec![Minion::new("X", -1, 1)]);
        assert_eq!(neg.validate(), Err(BoardError::NegativeAttack(0)));
        assert!(Warband::new(vec![]).validate().is_ok());
    }
}
