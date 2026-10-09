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
/// Version of the file we keep on the PC.
const SAVED_FORMAT: u32 = 1;

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

/// One entry, keeping only the two fields we read; every other field is
/// skipped without being kept. A field that is not a string is None.
#[derive(Deserialize)]
struct RawCard {
    #[serde(default, deserialize_with = "string_or_none")]
    id: Option<String>,
    #[serde(default, deserialize_with = "string_or_none")]
    name: Option<String>,
}

fn string_or_none<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    Ok(match Value::deserialize(d)? {
        Value::String(text) => Some(text),
        _ => None,
    })
}

/// The usable (id, name) pairs, read one entry at a time so a file of
/// millions of tiny entries is refused before it fills memory.
struct Pairs(Vec<(String, String)>);

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
                    let (Some(id), Some(name)) = (card.id, card.name) else {
                        continue;
                    };
                    if let Some(name) = clean_name(&name).filter(|_| valid_id(&id)) {
                        pairs.push((id, name));
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
}

impl Catalog {
    /// Reads HearthstoneJSON's `cards.json` (already inflated).
    pub fn from_hearthstonejson(json: &[u8]) -> Result<Catalog, CatalogError> {
        let Pairs(pairs) = serde_json::from_slice(json).map_err(|_| CatalogError::Shape)?;
        Catalog::from_pairs(pairs.into_iter())
    }

    /// An id that appears twice with different names is dropped: showing
    /// either could be the wrong card.
    fn from_pairs(pairs: impl Iterator<Item = (String, String)>) -> Result<Catalog, CatalogError> {
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
        if names.len() > MAX_CARDS {
            return Err(CatalogError::TooMany);
        }
        if names.len() < MIN_CARDS {
            return Err(CatalogError::TooFew(names.len()));
        }
        Ok(Catalog { names })
    }

    pub fn name(&self, id: &str) -> Option<&str> {
        self.names.get(id).map(String::as_str)
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
        };
        serde_json::to_vec(&saved).unwrap_or_default()
    }

    /// The saved file goes through the same checks as a download: it is a
    /// file on disk anyone could have edited.
    pub fn from_json(bytes: &[u8]) -> Result<SavedCatalog, CatalogError> {
        let saved: Saved = serde_json::from_slice(bytes).map_err(|_| CatalogError::Shape)?;
        if saved.format != SAVED_FORMAT {
            return Err(CatalogError::Shape);
        }
        let pairs = saved
            .names
            .into_iter()
            .filter(|(id, _)| valid_id(id))
            .filter_map(|(id, name)| clean_name(&name).map(|n| (id, n)));
        let etag = saved.etag.filter(|e| valid_etag(e));
        Ok(SavedCatalog {
            catalog: Catalog::from_pairs(pairs)?,
            fetched_at: saved.fetched_at,
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
