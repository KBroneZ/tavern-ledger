//! Card names by card id, read from HearthstoneJSON's `cards.json` (D-038).
//! The file is untrusted input: anything that does not look like the
//! documented shape is refused whole, and a card whose id or name does not
//! pass the checks is left out (it then shows as its id, "unknown").

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::Read;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The real file has about 30,000 cards. Fewer than this means the answer
/// is not the card list (an error page, a different file).
pub const MIN_CARDS: usize = 1000;
pub const MAX_NAME_CHARS: usize = 100;
/// The real file is about 10 MB; this stops a gzip bomb.
pub const MAX_INFLATED: usize = 64 * 1024 * 1024;
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
}

impl fmt::Display for CatalogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CatalogError::Encoding => write!(f, "the card data could not be unpacked"),
            CatalogError::Shape => write!(f, "the card data is not in the expected format"),
            CatalogError::TooFew(n) => write!(f, "the card data has only {n} usable cards"),
        }
    }
}

/// A card id as the log and HearthstoneJSON write it: letters, digits and
/// `_`, at most 64 characters. Anything else is never looked up or fetched.
pub fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// A name we are willing to show: trimmed, 1 to 100 characters, no control
/// characters. Shown with `textContent` only, never as markup.
fn clean_name(name: &str) -> Option<String> {
    let name = name.trim();
    let chars = name.chars().count();
    let ok = (1..=MAX_NAME_CHARS).contains(&chars) && !name.chars().any(char::is_control);
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

#[derive(Deserialize)]
struct RawCard {
    id: Option<Value>,
    name: Option<Value>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Catalog {
    names: BTreeMap<String, String>,
}

impl Catalog {
    /// Reads HearthstoneJSON's `cards.json` (already inflated).
    pub fn from_hearthstonejson(json: &[u8]) -> Result<Catalog, CatalogError> {
        let raw: Vec<RawCard> = serde_json::from_slice(json).map_err(|_| CatalogError::Shape)?;
        let pairs = raw.into_iter().filter_map(|card| {
            let id = card.id?.as_str()?.to_string();
            let name = clean_name(card.name?.as_str()?)?;
            valid_id(&id).then_some((id, name))
        });
        Catalog::from_pairs(pairs)
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
        let etag = saved.etag.filter(|e| e.len() <= 200 && e.is_ascii());
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
            "NUMBER_NAME",
        ] {
            assert_eq!(catalog.name(id), None, "{id}");
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
