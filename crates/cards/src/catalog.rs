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
/// no hero links; both still load, without what they lack, and are fetched
/// again (T-307, T-214).
const SAVED_FORMAT: u32 = 3;
const NAMES_ONLY_FORMAT: u32 = 1;
const NO_HERO_LINKS_FORMAT: u32 = 2;
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
}

/// The database id of a hero card, the only kind a skin links to.
fn hero_dbf(card: &RawCard) -> Option<i64> {
    let dbf = card.dbf.filter(|d| (1..=MAX_DBF_ID).contains(d))?;
    (card.card_type.as_deref() == Some("HERO")).then_some(dbf)
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
    // A field that is there but malformed leaves the card out.
    let tribes = match (&card.races, &card.race) {
        (Some(races), _) => strings(races)?,
        (None, Some(race)) => vec![race.as_str()?.to_string()],
        (None, None) => Vec::new(),
    };
    clean_pool(PoolMinion {
        tier,
        tribes,
        duos_only: card.duos_only == Some(true),
    })
}

/// Checks a pool entry (also when it is read back from the PC).
fn clean_pool(minion: PoolMinion) -> Option<PoolMinion> {
    let ok = (1..=MAX_TIER).contains(&minion.tier)
        && minion.tribes.len() <= MAX_TRIBES
        && minion.tribes.iter().all(|t| valid_tribe(t));
    ok.then_some(minion)
}

/// The usable (id, name, pool data, hero database id) entries, read one
/// entry at a time so a file of millions of tiny entries is refused before
/// it fills memory.
struct Pairs(Vec<(String, String, Option<PoolMinion>, Option<i64>)>);

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
                    let dbf = hero_dbf(&card);
                    let (Some(id), Some(name)) = (card.id, card.name) else {
                        continue;
                    };
                    if let Some(name) = clean_name(&name).filter(|_| valid_id(&id)) {
                        pairs.push((id, name, pool, dbf));
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
}

impl Catalog {
    /// Reads HearthstoneJSON's `cards.json` (already inflated).
    pub fn from_hearthstonejson(json: &[u8]) -> Result<Catalog, CatalogError> {
        let Pairs(entries) = serde_json::from_slice(json).map_err(|_| CatalogError::Shape)?;
        let mut pool = BTreeMap::new();
        let mut heroes = Vec::new();
        let mut pairs = Vec::with_capacity(entries.len());
        for (id, name, minion, dbf) in entries {
            if let Some(minion) = minion {
                pool.insert(id.clone(), minion);
            }
            if let Some(dbf) = dbf {
                heroes.push((dbf, id.clone()));
            }
            pairs.push((id, name));
        }
        Catalog::from_parts(pairs.into_iter(), pool, heroes.into_iter())
    }

    /// An id that appears twice with different names is dropped: showing
    /// either could be the wrong card. Pool data and hero links are kept only
    /// for a card that keeps its name, and dropped whole if there is far too
    /// much of them. A database id given to two cards is dropped too.
    fn from_parts(
        pairs: impl Iterator<Item = (String, String)>,
        mut pool: BTreeMap<String, PoolMinion>,
        hero_ids: impl Iterator<Item = (i64, String)>,
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
        let mut heroes: BTreeMap<i64, String> = BTreeMap::new();
        let mut twice = BTreeSet::new();
        for (dbf, id) in hero_ids.filter(|(dbf, id)| {
            (1..=MAX_DBF_ID).contains(dbf) && valid_id(id) && names.contains_key(id)
        }) {
            match heroes.get(&dbf) {
                Some(seen) if *seen != id => {
                    twice.insert(dbf);
                }
                Some(_) => {}
                None => {
                    heroes.insert(dbf, id);
                }
            }
        }
        heroes.retain(|dbf, _| !twice.contains(dbf));
        if heroes.len() > MAX_HEROES {
            heroes.clear();
        }
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
        })
    }

    /// The Battlegrounds pool minions by card id (empty when the data has
    /// none, e.g. a copy saved before T-307).
    pub fn pool(&self) -> &BTreeMap<String, PoolMinion> {
        &self.pool
    }

    pub fn name(&self, id: &str) -> Option<&str> {
        self.names.get(id).map(String::as_str)
    }

    /// Hero card ids by database id (empty when the data has none, e.g. a
    /// copy saved before T-214).
    pub fn hero_links(&self) -> &BTreeMap<i64, String> {
        &self.heroes
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
        };
        serde_json::to_vec(&saved).unwrap_or_default()
    }

    /// The saved file goes through the same checks as a download: it is a
    /// file on disk anyone could have edited.
    pub fn from_json(bytes: &[u8]) -> Result<SavedCatalog, CatalogError> {
        let saved: Saved = serde_json::from_slice(bytes).map_err(|_| CatalogError::Shape)?;
        // An older copy lacks what later formats added: fetched again soon.
        let old = saved.format == NAMES_ONLY_FORMAT || saved.format == NO_HERO_LINKS_FORMAT;
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
        // An older copy has no pool or no hero links: without its ETag the
        // next fetch brings the whole file instead of "unchanged".
        let etag = saved.etag.filter(|e| valid_etag(e)).filter(|_| !old);
        Ok(SavedCatalog {
            catalog: Catalog::from_parts(pairs, pool, saved.heroes.into_iter())?,
            // An older copy counts as old, so it is replaced soon.
            fetched_at: if old { 0 } else { saved.fetched_at },
            etag,
        })
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Write;

    /// A made-up card list with `n` cards (`CARD_0` … named `Card 0` …) plus
    /// the extra entries given. No real card data in the tests (D-038).
    pub fn synthetic(n: usize, extra: Vec<Value>) -> Vec<u8> {
        let mut cards: Vec<Value> = (0..n)
            .map(|i| json!({"id": format!("CARD_{i}"), "name": format!("Card {i}"), "type": "MINION"}))
            .collect();
        cards.extend(extra);
        serde_json::to_vec(&cards).unwrap()
    }

    pub fn gzip(bytes: &[u8]) -> Vec<u8> {
        let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        enc.write_all(bytes).unwrap();
        enc.finish().unwrap()
    }

    #[test]
    fn names_are_read_by_card_id() {
        let extra = vec![json!({"id": "HERO_X", "name": "  Made-up Hero  ", "type": "HERO"})];
        let catalog = Catalog::from_hearthstonejson(&synthetic(MIN_CARDS, extra)).unwrap();
        assert_eq!(catalog.name("CARD_7"), Some("Card 7"));
        assert_eq!(catalog.name("HERO_X"), Some("Made-up Hero"), "trimmed");
        assert_eq!(catalog.name("NOT_THERE"), None);
        assert_eq!(catalog.len(), MIN_CARDS + 1);
    }

    #[test]
    fn anything_but_an_array_of_cards_is_refused_whole() {
        for body in [
            b"<html>Error</html>".to_vec(),
            b"{\"cards\": []}".to_vec(),
            b"[1, 2, 3]".to_vec(),
            b"".to_vec(),
        ] {
            assert_eq!(
                Catalog::from_hearthstonejson(&body),
                Err(CatalogError::Shape)
            );
        }
    }

    #[test]
    fn a_short_list_is_not_the_card_data() {
        assert_eq!(
            Catalog::from_hearthstonejson(&synthetic(10, vec![])),
            Err(CatalogError::TooFew(10))
        );
    }

    #[test]
    fn cards_with_a_bad_id_or_name_are_left_out() {
        let long = "x".repeat(MAX_NAME_CHARS + 1);
        let extra = vec![
            json!({"id": "BAD ID", "name": "Space in id"}),
            json!({"id": "../ESCAPE", "name": "Path in id"}),
            json!({"id": "NO_NAME"}),
            json!({"id": "EMPTY_NAME", "name": "   "}),
            json!({"id": "LONG_NAME", "name": long}),
            json!({"id": "CONTROL", "name": "Bell\u{7}"}),
            json!({"id": "BIDI", "name": "Evil\u{202E}lacol"}),
            json!({"id": "ZERO_WIDTH", "name": "Zero\u{200B}Width"}),
            json!({"id": "NUMBER_NAME", "name": 5}),
            json!({"id": 42, "name": "Number id"}),
            json!({"name": "No id"}),
        ];
        let catalog = Catalog::from_hearthstonejson(&synthetic(MIN_CARDS, extra)).unwrap();
        assert_eq!(catalog.len(), MIN_CARDS);
        for id in [
            "BAD ID",
            "../ESCAPE",
            "NO_NAME",
            "EMPTY_NAME",
            "LONG_NAME",
            "CONTROL",
            "BIDI",
            "ZERO_WIDTH",
            "NUMBER_NAME",
        ] {
            assert_eq!(catalog.name(id), None, "{id}");
        }
    }

    #[test]
    fn a_file_of_millions_of_entries_or_cards_is_refused() {
        let empties = format!("[{}{{}}]", "{},".repeat(MAX_ENTRIES));
        assert_eq!(
            Catalog::from_hearthstonejson(empties.as_bytes()),
            Err(CatalogError::Shape)
        );
        let many = synthetic(MAX_CARDS + 1, vec![]);
        assert_eq!(
            Catalog::from_hearthstonejson(&many),
            Err(CatalogError::TooMany)
        );
    }

    #[test]
    fn only_visible_ascii_etags_are_sent_back() {
        assert!(valid_etag(r#""a7fd90b783d1ce0883662523f079c7f4-2""#));
        assert!(valid_etag(r#"W/"x""#));
        for bad in ["", "a\nb", "caf\u{e9}", &"x".repeat(201)] {
            assert!(!valid_etag(bad), "{bad:?}");
        }
    }

    #[test]
    fn an_id_with_two_different_names_is_dropped_not_guessed() {
        let extra = vec![
            json!({"id": "TWICE", "name": "First"}),
            json!({"id": "TWICE", "name": "Second"}),
            json!({"id": "SAME", "name": "Same"}),
            json!({"id": "SAME", "name": "Same"}),
        ];
        let catalog = Catalog::from_hearthstonejson(&synthetic(MIN_CARDS, extra)).unwrap();
        assert_eq!(catalog.name("TWICE"), None);
        assert_eq!(catalog.name("SAME"), Some("Same"));
    }

    #[test]
    fn pool_minions_keep_their_tier_and_tribes() {
        let extra = vec![
            json!({"id": "BG_A", "name": "A", "isBattlegroundsPoolMinion": true, "techLevel": 1, "races": ["BEAST"], "race": "BEAST"}),
            json!({"id": "BG_B", "name": "B", "isBattlegroundsPoolMinion": true, "techLevel": 2, "races": ["MURLOC", "PIRATE"]}),
            json!({"id": "BG_N", "name": "N", "isBattlegroundsPoolMinion": true, "techLevel": 1}),
            json!({"id": "BG_OLD", "name": "Old", "isBattlegroundsPoolMinion": true, "techLevel": 3, "race": "DEMON"}),
            json!({"id": "BG_D", "name": "D", "isBattlegroundsPoolMinion": true, "techLevel": 1, "isBattlegroundsDuosExclusive": true}),
            json!({"id": "BG_OUT", "name": "Out", "isBattlegroundsPoolMinion": false, "techLevel": 1}),
            json!({"id": "BG_NOT", "name": "Not", "techLevel": 1}),
        ];
        let catalog = Catalog::from_hearthstonejson(&synthetic(MIN_CARDS, extra)).unwrap();
        let pool = catalog.pool();
        let minion = |tier: u8, tribes: &[&str], duos_only: bool| PoolMinion {
            tier,
            tribes: tribes.iter().map(|t| t.to_string()).collect(),
            duos_only,
        };
        assert_eq!(pool["BG_A"], minion(1, &["BEAST"], false));
        assert_eq!(pool["BG_B"], minion(2, &["MURLOC", "PIRATE"], false));
        assert_eq!(pool["BG_N"], minion(1, &[], false), "neutral");
        assert_eq!(
            pool["BG_OLD"],
            minion(3, &["DEMON"], false),
            "the old single race"
        );
        assert_eq!(pool["BG_D"], minion(1, &[], true));
        assert!(!pool.contains_key("BG_OUT") && !pool.contains_key("BG_NOT"));
        assert!(
            !pool.contains_key("CARD_1"),
            "an ordinary card is not in the pool"
        );
    }

    #[test]
    fn pool_minions_with_odd_fields_are_never_listed() {
        let extra = vec![
            json!({"id": "T0", "name": "x", "isBattlegroundsPoolMinion": true, "techLevel": 0}),
            json!({"id": "T8", "name": "x", "isBattlegroundsPoolMinion": true, "techLevel": 8}),
            json!({"id": "TS", "name": "x", "isBattlegroundsPoolMinion": true, "techLevel": "1"}),
            json!({"id": "TN", "name": "x", "isBattlegroundsPoolMinion": true}),
            json!({"id": "RL", "name": "x", "isBattlegroundsPoolMinion": true, "techLevel": 1, "races": ["beast"]}),
            json!({"id": "RN", "name": "x", "isBattlegroundsPoolMinion": true, "techLevel": 1, "races": ["BEAST", 5]}),
            json!({"id": "RM", "name": "x", "isBattlegroundsPoolMinion": true, "techLevel": 1, "races": ["A", "B", "C", "D", "E"]}),
            json!({"id": "RX", "name": "x", "isBattlegroundsPoolMinion": true, "techLevel": 1, "races": ["<b>"]}),
            json!({"id": "RO", "name": "x", "isBattlegroundsPoolMinion": true, "techLevel": 1, "races": "BEAST"}),
            json!({"id": "R1", "name": "x", "isBattlegroundsPoolMinion": true, "techLevel": 1, "race": 7}),
            json!({"id": "BAD ID", "name": "x", "isBattlegroundsPoolMinion": true, "techLevel": 1}),
        ];
        let catalog = Catalog::from_hearthstonejson(&synthetic(MIN_CARDS, extra)).unwrap();
        assert!(catalog.pool().is_empty(), "{:?}", catalog.pool());
        assert_eq!(catalog.name("RN"), Some("x"), "the name still shows");
    }

    #[test]
    fn the_pool_is_saved_and_checked_again_when_read() {
        let extra = vec![
            json!({"id": "BG_A", "name": "A", "isBattlegroundsPoolMinion": true, "techLevel": 1, "races": ["BEAST"]}),
        ];
        let catalog = Catalog::from_hearthstonejson(&synthetic(MIN_CARDS, extra)).unwrap();
        let saved = SavedCatalog {
            catalog,
            fetched_at: 1_700_000_000,
            etag: Some("\"abc\"".into()),
        };
        let back = SavedCatalog::from_json(&saved.to_json()).unwrap();
        assert_eq!(back, saved);
        let mut edited: Value = serde_json::from_slice(&saved.to_json()).unwrap();
        edited["pool"]["BG_A"]["tier"] = json!(9);
        edited["pool"]["NO_NAME"] = json!({"tier": 1, "tribes": []});
        let back = SavedCatalog::from_json(&serde_json::to_vec(&edited).unwrap()).unwrap();
        assert!(back.catalog.pool().is_empty());
    }

    #[test]
    fn a_names_only_copy_still_loads_and_is_replaced_soon() {
        let catalog = Catalog::from_hearthstonejson(&synthetic(MIN_CARDS, vec![])).unwrap();
        let saved = SavedCatalog {
            catalog,
            fetched_at: 1_700_000_000,
            etag: Some("\"abc\"".into()),
        };
        let mut old: Value = serde_json::from_slice(&saved.to_json()).unwrap();
        old["format"] = json!(1);
        old.as_object_mut().unwrap().remove("pool");
        let back = SavedCatalog::from_json(&serde_json::to_vec(&old).unwrap()).unwrap();
        assert_eq!(back.catalog.name("CARD_1"), Some("Card 1"));
        assert!(back.catalog.pool().is_empty());
        assert_eq!(back.fetched_at, 0, "old: fetched again at the next chance");
        assert_eq!(back.etag, None, "the whole file, not \"unchanged\"");
    }

    #[test]
    fn hero_cards_are_found_by_their_database_id() {
        let extra = vec![
            json!({"id": "HERO_BASE", "name": "Base", "type": "HERO", "dbfId": 59999}),
            json!({"id": "HERO_BASE_SKIN_A", "name": "Base", "type": "HERO", "dbfId": 70001}),
            json!({"id": "MINION_X", "name": "Minion", "type": "MINION", "dbfId": 12}),
            json!({"id": "HERO_NO_DBF", "name": "No id", "type": "HERO"}),
            json!({"id": "HERO_BAD_DBF", "name": "Bad id", "type": "HERO", "dbfId": "5"}),
            json!({"id": "HERO_ZERO", "name": "Zero", "type": "HERO", "dbfId": 0}),
            json!({"id": "HERO_TWIN_A", "name": "Twin A", "type": "HERO", "dbfId": 333}),
            json!({"id": "HERO_TWIN_B", "name": "Twin B", "type": "HERO", "dbfId": 333}),
            json!({"id": "BAD ID", "name": "Bad", "type": "HERO", "dbfId": 444}),
        ];
        let catalog = Catalog::from_hearthstonejson(&synthetic(MIN_CARDS, extra)).unwrap();
        let links: Vec<_> = catalog
            .hero_links()
            .iter()
            .map(|(d, id)| (*d, id.as_str()))
            .collect();
        // Only heroes; a database id two cards claim is dropped, not guessed.
        assert_eq!(links, [(59999, "HERO_BASE"), (70001, "HERO_BASE_SKIN_A")]);
    }

    #[test]
    fn hero_links_are_saved_and_checked_again_when_read() {
        let extra =
            vec![json!({"id": "HERO_BASE", "name": "Base", "type": "HERO", "dbfId": 59999})];
        let catalog = Catalog::from_hearthstonejson(&synthetic(MIN_CARDS, extra)).unwrap();
        let saved = SavedCatalog {
            catalog,
            fetched_at: 1_700_000_000,
            etag: Some("\"abc\"".into()),
        };
        let back = SavedCatalog::from_json(&saved.to_json()).unwrap();
        assert_eq!(back, saved);
        let mut edited: Value = serde_json::from_slice(&saved.to_json()).unwrap();
        edited["heroes"]["7"] = json!("../EVIL");
        edited["heroes"]["8"] = json!("NOT_A_CARD");
        edited["heroes"]["-1"] = json!("HERO_BASE");
        let back = SavedCatalog::from_json(&serde_json::to_vec(&edited).unwrap()).unwrap();
        assert_eq!(back.catalog.hero_links().len(), 1);
        // A copy from before the links loads and is fetched again soon.
        let mut old: Value = serde_json::from_slice(&saved.to_json()).unwrap();
        old["format"] = json!(2);
        old.as_object_mut().unwrap().remove("heroes");
        let back = SavedCatalog::from_json(&serde_json::to_vec(&old).unwrap()).unwrap();
        assert!(back.catalog.hero_links().is_empty());
        assert_eq!((back.fetched_at, back.etag), (0, None));
        assert_eq!(back.catalog.name("HERO_BASE"), Some("Base"));
    }

    #[test]
    fn card_ids_are_plain() {
        assert!(valid_id("BG_CFM_315"));
        assert!(valid_id("TB_BaconShop_HERO_01"));
        for bad in [
            "",
            "a/b",
            "a\\b",
            "..",
            "a.jpg",
            "a b",
            "a%2F",
            &"A".repeat(65),
        ] {
            assert!(!valid_id(bad), "{bad:?}");
        }
    }

    #[test]
    fn gzip_is_inflated_and_garbage_is_refused() {
        let body = synthetic(3, vec![]);
        assert_eq!(gunzip(&gzip(&body)).unwrap(), body);
        assert_eq!(gunzip(b"not gzip"), Err(CatalogError::Encoding));
    }

    #[test]
    fn a_saved_catalog_reads_back_and_is_checked_again() {
        let catalog = Catalog::from_hearthstonejson(&synthetic(MIN_CARDS + 1, vec![])).unwrap();
        let saved = SavedCatalog {
            catalog,
            fetched_at: 1_700_000_000,
            etag: Some("\"abc\"".into()),
        };
        assert_eq!(SavedCatalog::from_json(&saved.to_json()).unwrap(), saved);

        let mut edited: Value = serde_json::from_slice(&saved.to_json()).unwrap();
        edited["names"]["../EVIL"] = json!("Evil");
        edited["names"]["CARD_1"] = json!("\u{1b}[31m");
        let back = SavedCatalog::from_json(&serde_json::to_vec(&edited).unwrap()).unwrap();
        assert_eq!(back.catalog.name("../EVIL"), None);
        assert_eq!(back.catalog.name("CARD_1"), None);

        edited["format"] = json!(99);
        assert!(SavedCatalog::from_json(&serde_json::to_vec(&edited).unwrap()).is_err());
        assert!(SavedCatalog::from_json(b"{broken").is_err());
    }
}
