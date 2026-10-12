//! Card names by card id, read from HearthstoneJSON's `cards.json` (D-038).
//! The file is untrusted input: anything that does not look like the
//! documented shape is refused whole, and a card whose id or name does not
//! pass the checks is left out (it then shows as its id, "unknown").

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::Read;

use serde::de::{self, Deserializer, SeqAccess, Visitor};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::combat_facts::{CombatFacts, MAX_COMBAT_CARDS};

/// The real file has about 30,000 cards. Fewer than this means the answer
/// is not the card list (an error page, a different file).
pub const MIN_CARDS: usize = 1000;
pub const MAX_NAME_CHARS: usize = 100;
/// More than this is not the card list either (and would cost memory).
pub const MAX_CARDS: usize = 100_000;
/// Entries read before the file is refused, usable or not.
pub const MAX_ENTRIES: usize = 200_000;
/// The real file is about 10 MB; this stops a gzip bomb.
pub const MAX_INFLATED: usize = 32 * 1024 * 1024;
/// Version of the file we keep on the PC. Format 1 had names only, format 2
/// no hero links, format 3 no card links, format 4 no combat facts, format 5
/// no skin parents; all still load, without what they lack, and are fetched
/// again (T-307, T-214, T-207, T-605, T-540).
const SAVED_FORMAT: u32 = 6;
const NAMES_ONLY_FORMAT: u32 = 1;
const NO_HERO_LINKS_FORMAT: u32 = 2;
const NO_CARD_LINKS_FORMAT: u32 = 3;
const NO_COMBAT_FACTS_FORMAT: u32 = 4;
const NO_SKIN_PARENTS_FORMAT: u32 = 5;
/// The real data has about 3,000 hero cards (skins included).
pub const MAX_HEROES: usize = 20_000;
/// Database ids are positive and far below this.
pub const MAX_DBF_ID: i64 = 10_000_000;
/// The real pool has about 300 minions.
pub const MAX_POOL: usize = 2_000;
/// Battlegrounds tavern tiers are 1 to 7.
pub const MAX_TIER: u8 = 7;
/// A pool minion has at most a couple of tribes; more is not the real data.
pub const MAX_TRIBES: usize = 4;
const MAX_TRIBE_CHARS: usize = 32;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CatalogError {
    /// Not gzip when it said so, or larger than `MAX_INFLATED`.
    Encoding,
    /// Not a JSON array of card objects.
    Shape,
    /// The array had fewer usable cards than `MIN_CARDS`.
    TooFew(usize),
    /// More than `MAX_CARDS` usable cards.
    TooMany,
}

impl fmt::Display for CatalogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CatalogError::Encoding => write!(f, "the card data could not be unpacked"),
            CatalogError::Shape => write!(f, "the card data is not in the expected format"),
            CatalogError::TooFew(n) => write!(f, "the card data has only {n} usable cards"),
            CatalogError::TooMany => write!(f, "the card data has far more cards than expected"),
        }
    }
}

/// A card id as the log and HearthstoneJSON write it: letters, digits and
/// `_`, at most 64 characters. Anything else is never looked up or fetched.
pub fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// Invisible characters that change how text reads (bidirectional
/// overrides, zero-width marks): a name with one could pass for another.
fn is_format_char(c: char) -> bool {
    matches!(c, '\u{00AD}' | '\u{061C}' | '\u{180E}' | '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2060}'..='\u{206F}' | '\u{FEFF}')
}

/// A name we are willing to show: trimmed, 1 to 100 characters, no control
/// or invisible formatting characters. Shown with `textContent` only, never
/// as markup.
fn clean_name(name: &str) -> Option<String> {
    let name = name.trim();
    let chars = name.chars().count();
    let ok = (1..=MAX_NAME_CHARS).contains(&chars)
        && !name.chars().any(|c| c.is_control() || is_format_char(c));
    ok.then(|| name.to_string())
}

/// Inflates a gzip body, refusing anything larger than `MAX_INFLATED`.
pub fn gunzip(body: &[u8]) -> Result<Vec<u8>, CatalogError> {
    let mut out = Vec::new();
    flate2::read::GzDecoder::new(body)
        .take(MAX_INFLATED as u64 + 1)
        .read_to_end(&mut out)
        .map_err(|_| CatalogError::Encoding)?;
    if out.len() > MAX_INFLATED {
        return Err(CatalogError::Encoding);
    }
    Ok(out)
}

/// One entry, keeping only the fields we read; every other field is skipped
/// without being kept. A field of the wrong type is None.
#[derive(Deserialize)]
struct RawCard {
    #[serde(default, deserialize_with = "string_or_none")]
    id: Option<String>,
    #[serde(default, deserialize_with = "string_or_none")]
    name: Option<String>,
    /// Battlegrounds fields (T-307), as HearthstoneJSON writes them.
    #[serde(
        default,
        rename = "isBattlegroundsPoolMinion",
        deserialize_with = "bool_or_none"
    )]
    pool: Option<bool>,
    #[serde(default, rename = "techLevel", deserialize_with = "int_or_none")]
    tier: Option<i64>,
    /// Kept raw: a field that is there but malformed must not read as
    /// "no tribe" (a neutral minion).
    #[serde(default)]
    races: Option<Value>,
    #[serde(default)]
    race: Option<Value>,
    #[serde(
        default,
        rename = "isBattlegroundsDuosExclusive",
        deserialize_with = "bool_or_none"
    )]
    duos_only: Option<bool>,
    /// The database id the log uses for links such as a skin's base hero
    /// (`BACON_SKIN_PARENT_ID`, T-214).
    #[serde(default, rename = "dbfId", deserialize_with = "int_or_none")]
    dbf: Option<i64>,
    #[serde(default, rename = "type", deserialize_with = "string_or_none")]
    card_type: Option<String>,
    /// A Battlegrounds skin's base hero, by database id (T-540).
    #[serde(
        default,
        rename = "battlegroundsSkinParentId",
        deserialize_with = "int_or_none"
    )]
    skin_parent: Option<i64>,
    /// Combat facts (T-605).
    #[serde(default, deserialize_with = "string_or_none")]
    set: Option<String>,
    #[serde(default, deserialize_with = "int_or_none")]
    attack: Option<i64>,
    #[serde(default, deserialize_with = "int_or_none")]
    health: Option<i64>,
    #[serde(default)]
    mechanics: Option<Value>,
}

/// A card's database id, when it is a plain one.
fn card_dbf(card: &RawCard) -> Option<i64> {
    card.dbf.filter(|d| (1..=MAX_DBF_ID).contains(d))
}

/// The database id of a hero card, the only kind a skin links to.
fn hero_dbf(card: &RawCard) -> Option<i64> {
    card_dbf(card).filter(|_| card.card_type.as_deref() == Some("HERO"))
}

/// The base hero's database id of a hero skin, when it is a plain one.
fn skin_parent(card: &RawCard) -> Option<i64> {
    card.skin_parent
        .filter(|d| (1..=MAX_DBF_ID).contains(d))
        .filter(|_| card.card_type.as_deref() == Some("HERO"))
}

fn string_or_none<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    Ok(match Value::deserialize(d)? {
        Value::String(text) => Some(text),
        _ => None,
    })
}

fn bool_or_none<'de, D: Deserializer<'de>>(d: D) -> Result<Option<bool>, D::Error> {
    Ok(Value::deserialize(d)?.as_bool())
}

fn int_or_none<'de, D: Deserializer<'de>>(d: D) -> Result<Option<i64>, D::Error> {
    Ok(Value::deserialize(d)?.as_i64())
}

/// A list of strings, or None if it is anything else (one item that is not
/// a string spoils the list).
fn strings(value: &Value) -> Option<Vec<String>> {
    value
        .as_array()?
        .iter()
        .map(|v| v.as_str().map(String::from))
        .collect()
}

/// A Battlegrounds pool minion: its tavern tier, its tribes as the log
/// names them (`BEAST`, `ALL`; empty for a neutral minion) and whether only
/// Duos has it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PoolMinion {
    pub tier: u8,
    pub tribes: Vec<String>,
    #[serde(default)]
    pub duos_only: bool,
}

/// A tribe name we keep: upper-case letters and `_`, as the log writes them.
fn valid_tribe(tribe: &str) -> bool {
    !tribe.is_empty()
        && tribe.len() <= MAX_TRIBE_CHARS
        && tribe.bytes().all(|b| b.is_ascii_uppercase() || b == b'_')
}

/// The pool data of a card, or None when it is not a pool minion or its
/// fields do not pass the checks (it then is never listed as possible).
fn pool_minion(card: &RawCard) -> Option<PoolMinion> {
    if card.pool != Some(true) {
        return None;
    }
    let tier = u8::try_from(card.tier?).ok()?;
    let tribes = card_tribes(card)?;
    clean_pool(PoolMinion {
        tier,
        tribes,
        duos_only: card.duos_only == Some(true),
    })
}

/// A card's types; None when a field is there but malformed (the card is
/// then left out, not read as having no type).
fn card_tribes(card: &RawCard) -> Option<Vec<String>> {
    Some(match (&card.races, &card.race) {
        (Some(races), _) => strings(races)?,
        (None, Some(race)) => vec![race.as_str()?.to_string()],
        (None, None) => Vec::new(),
    })
}

fn combat_facts(card: &RawCard) -> Option<CombatFacts> {
    CombatFacts::from_card(
        card.set.as_deref(),
        card.card_type.as_deref(),
        card.attack,
        card.health,
        card.tier,
        card_tribes(card),
        card.mechanics.as_ref(),
    )
}

/// Checks a pool entry (also when it is read back from the PC).
fn clean_pool(minion: PoolMinion) -> Option<PoolMinion> {
    let ok = (1..=MAX_TIER).contains(&minion.tier)
        && minion.tribes.len() <= MAX_TRIBES
        && minion.tribes.iter().all(|t| valid_tribe(t));
    ok.then_some(minion)
}

/// One usable entry.
struct Entry {
    id: String,
    name: String,
    pool: Option<PoolMinion>,
    hero: Option<i64>,
    dbf: Option<i64>,
    skin_parent: Option<i64>,
    combat: Option<CombatFacts>,
}

/// The usable entries, read one entry at a time so a file of millions of
/// tiny entries is refused before it fills memory.
struct Pairs(Vec<Entry>);

impl<'de> Deserialize<'de> for Pairs {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Pairs, D::Error> {
        struct Entries;
        impl<'de> Visitor<'de> for Entries {
            type Value = Pairs;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("an array of cards")
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Pairs, A::Error> {
                let mut pairs = Vec::new();
                let mut read = 0usize;
                while let Some(card) = seq.next_element::<RawCard>()? {
                    read += 1;
                    if read > MAX_ENTRIES {
                        return Err(de::Error::custom("too many entries"));
                    }
                    let pool = pool_minion(&card);
                    let hero = hero_dbf(&card);
                    let dbf = card_dbf(&card);
                    let skin_parent = skin_parent(&card);
                    let combat = combat_facts(&card);
                    let (Some(id), Some(name)) = (card.id, card.name) else {
                        continue;
                    };
                    if let Some(name) = clean_name(&name).filter(|_| valid_id(&id)) {
                        pairs.push(Entry {
                            id,
                            name,
                            pool,
                            hero,
                            dbf,
                            skin_parent,
                            combat,
                        });
                    }
                }
                Ok(Pairs(pairs))
            }
        }
        d.deserialize_seq(Entries)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Catalog {
    names: BTreeMap<String, String>,
    /// Battlegrounds pool minions by card id; every id here has a name.
    pool: BTreeMap<String, PoolMinion>,
    /// Hero card ids by database id (T-214); every id here has a name.
    heroes: BTreeMap<i64, String>,
    /// Every card id by database id, for the trinkets and the Old God the log
    /// gives as database ids (T-207); every id here has a name.
    cards: BTreeMap<i64, String>,
    /// Each hero skin's base hero as a database id (`battlegroundsSkinParentId`,
    /// T-540); every id here has a name.
    skin_parents: BTreeMap<String, i64>,
    /// Battlegrounds minions' combat facts by card id (T-605); every id here
    /// has a name.
    combat: BTreeMap<String, CombatFacts>,
}

/// Card ids by database id, keeping only ids that have a name. A database id
/// two cards claim is dropped, not guessed; far too many drops them all.
fn links(
    ids: impl Iterator<Item = (i64, String)>,
    names: &BTreeMap<String, String>,
    max: usize,
) -> BTreeMap<i64, String> {
    let mut links: BTreeMap<i64, String> = BTreeMap::new();
    let mut twice = BTreeSet::new();
    for (dbf, id) in ids.filter(|(dbf, id)| {
        (1..=MAX_DBF_ID).contains(dbf) && valid_id(id) && names.contains_key(id)
    }) {
        match links.get(&dbf) {
            Some(seen) if *seen != id => {
                twice.insert(dbf);
            }
            Some(_) => {}
            None => {
                links.insert(dbf, id);
            }
        }
    }
    links.retain(|dbf, _| !twice.contains(dbf));
    if links.len() > max {
        links.clear();
    }
    links
}

impl Catalog {
    /// Reads HearthstoneJSON's `cards.json` (already inflated).
    pub fn from_hearthstonejson(json: &[u8]) -> Result<Catalog, CatalogError> {
        let Pairs(entries) = serde_json::from_slice(json).map_err(|_| CatalogError::Shape)?;
        let mut pool = BTreeMap::new();
        let mut heroes = Vec::new();
        let mut cards = Vec::new();
        let mut skin_parents = BTreeMap::new();
        let mut combat = BTreeMap::new();
        let mut pairs = Vec::with_capacity(entries.len());
        for e in entries {
            if let Some(minion) = e.pool {
                pool.insert(e.id.clone(), minion);
            }
            if let Some(hero) = e.hero {
                heroes.push((hero, e.id.clone()));
            }
            if let Some(dbf) = e.dbf {
                cards.push((dbf, e.id.clone()));
            }
            if let Some(parent) = e.skin_parent {
                skin_parents.insert(e.id.clone(), parent);
            }
            if let Some(facts) = e.combat {
                combat.insert(e.id.clone(), facts);
            }
            pairs.push((e.id, e.name));
        }
        Catalog::from_parts(
            pairs.into_iter(),
            pool,
            heroes.into_iter(),
            cards.into_iter(),
            skin_parents,
            combat,
        )
    }

    /// An id that appears twice with different names is dropped: showing
    /// either could be the wrong card. Pool data and hero links are kept only
    /// for a card that keeps its name, and dropped whole if there is far too
    /// much of them. A database id given to two cards is dropped too.
    fn from_parts(
        pairs: impl Iterator<Item = (String, String)>,
        mut pool: BTreeMap<String, PoolMinion>,
        hero_ids: impl Iterator<Item = (i64, String)>,
        card_ids: impl Iterator<Item = (i64, String)>,
        mut skin_parents: BTreeMap<String, i64>,
        mut combat: BTreeMap<String, CombatFacts>,
    ) -> Result<Catalog, CatalogError> {
        let mut names = BTreeMap::new();
        let mut clashes = BTreeSet::new();
        for (id, name) in pairs {
            match names.get(&id) {
                Some(seen) if *seen != name => {
                    clashes.insert(id);
                }
                Some(_) => {}
                None => {
                    names.insert(id, name);
                }
            }
        }
        names.retain(|id, _| !clashes.contains(id));
        pool.retain(|id, _| names.contains_key(id));
        if pool.len() > MAX_POOL {
            pool.clear();
        }
        combat.retain(|id, _| names.contains_key(id));
        if combat.len() > MAX_COMBAT_CARDS {
            combat.clear();
        }
        skin_parents.retain(|id, dbf| names.contains_key(id) && (1..=MAX_DBF_ID).contains(dbf));
        if skin_parents.len() > MAX_HEROES {
            skin_parents.clear();
        }
        let heroes = links(hero_ids, &names, MAX_HEROES);
        let cards = links(card_ids, &names, MAX_CARDS);
        if names.len() > MAX_CARDS {
            return Err(CatalogError::TooMany);
        }
        if names.len() < MIN_CARDS {
            return Err(CatalogError::TooFew(names.len()));
        }
        Ok(Catalog {
            names,
            pool,
            heroes,
            cards,
            skin_parents,
            combat,
        })
    }

    /// The Battlegrounds pool minions by card id (empty when the data has
    /// none, e.g. a copy saved before T-307).
    pub fn pool(&self) -> &BTreeMap<String, PoolMinion> {
        &self.pool
    }

    /// A Battlegrounds minion's combat facts (T-605); None when the data
    /// has none for it (e.g. a copy saved before), never a guess.
    pub fn combat_facts(&self, id: &str) -> Option<&CombatFacts> {
        self.combat.get(id)
    }

    pub fn name(&self, id: &str) -> Option<&str> {
        self.names.get(id).map(String::as_str)
    }

    /// Hero card ids by database id (empty when the data has none, e.g. a
    /// copy saved before T-214).
    pub fn hero_links(&self) -> &BTreeMap<i64, String> {
        &self.heroes
    }

    /// Each hero skin's base hero as a database id (empty when the data has
    /// none, e.g. a copy saved before T-540).
    pub fn skin_parents(&self) -> &BTreeMap<String, i64> {
        &self.skin_parents
    }

    /// The card id of a database id; None when the data has none (e.g. a
    /// copy saved before T-207) or two cards claim it.
    pub fn card_of_dbf(&self, dbf: i64) -> Option<&str> {
        self.cards.get(&dbf).map(String::as_str)
    }

    pub fn contains(&self, id: &str) -> bool {
        self.names.contains_key(id)
    }

    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}

/// What we keep on the PC (`card-cache/catalog.json`): the names only, when
/// they were fetched and the server's ETag. Never in the repo (D-038).
#[derive(Serialize, Deserialize)]
struct Saved {
    format: u32,
    fetched_at: u64,
    etag: Option<String>,
    names: BTreeMap<String, String>,
    #[serde(default)]
    pool: BTreeMap<String, PoolMinion>,
    #[serde(default)]
    heroes: BTreeMap<i64, String>,
    #[serde(default)]
    cards: BTreeMap<i64, String>,
    #[serde(default)]
    skin_parents: BTreeMap<String, i64>,
    #[serde(default)]
    combat: BTreeMap<String, CombatFacts>,
}

/// An ETag we send back as-is: visible ASCII only, so a hand-edited file
/// cannot make every request fail.
pub fn valid_etag(etag: &str) -> bool {
    !etag.is_empty() && etag.len() <= 200 && etag.bytes().all(|b| (0x20..0x7f).contains(&b))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavedCatalog {
    pub catalog: Catalog,
    /// Seconds since 1970 when the server last confirmed it.
    pub fetched_at: u64,
    pub etag: Option<String>,
}

impl SavedCatalog {
    pub fn to_json(&self) -> Vec<u8> {
        let saved = Saved {
            format: SAVED_FORMAT,
            fetched_at: self.fetched_at,
            etag: self.etag.clone(),
            names: self.catalog.names.clone(),
            pool: self.catalog.pool.clone(),
            heroes: self.catalog.heroes.clone(),
            cards: self.catalog.cards.clone(),
            skin_parents: self.catalog.skin_parents.clone(),
            combat: self.catalog.combat.clone(),
        };
        serde_json::to_vec(&saved).unwrap_or_default()
    }

    /// The saved file goes through the same checks as a download: it is a
    /// file on disk anyone could have edited.
    pub fn from_json(bytes: &[u8]) -> Result<SavedCatalog, CatalogError> {
        let saved: Saved = serde_json::from_slice(bytes).map_err(|_| CatalogError::Shape)?;
        // An older copy lacks what later formats added: fetched again soon.
        let old = [
            NAMES_ONLY_FORMAT,
            NO_HERO_LINKS_FORMAT,
            NO_CARD_LINKS_FORMAT,
            NO_COMBAT_FACTS_FORMAT,
            NO_SKIN_PARENTS_FORMAT,
        ]
        .contains(&saved.format);
        if saved.format != SAVED_FORMAT && !old {
            return Err(CatalogError::Shape);
        }
        let pairs = saved
            .names
            .into_iter()
            .filter(|(id, _)| valid_id(id))
            .filter_map(|(id, name)| clean_name(&name).map(|n| (id, n)));
        let pool = saved
            .pool
            .into_iter()
            .filter(|(id, _)| valid_id(id))
            .filter_map(|(id, minion)| Some((id, clean_pool(minion)?)))
            .collect();
        let combat = saved
            .combat
            .into_iter()
            .filter(|(id, _)| valid_id(id))
            .filter_map(|(id, facts)| Some((id, facts.clean()?)))
            .collect();
        // An older copy has no pool or no hero links: without its ETag the
        // next fetch brings the whole file instead of "unchanged".
        let etag = saved.etag.filter(|e| valid_etag(e)).filter(|_| !old);
        Ok(SavedCatalog {
            catalog: Catalog::from_parts(
                pairs,
                pool,
                saved.heroes.into_iter(),
                saved.cards.into_iter(),
                saved
                    .skin_parents
                    .into_iter()
                    .filter(|(id, _)| valid_id(id))
                    .collect(),
                combat,
            )?,
            // An older copy counts as old, so it is replaced soon.
            fetched_at: if old { 0 } else { saved.fetched_at },
            etag,
        })
    }
}

#[cfg(test)]
#[path = "catalog_tests.rs"]
pub mod tests;
