//! Tests of `catalog` (moved out to keep `catalog.rs` under 800 lines).

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
fn any_card_is_found_by_its_database_id_and_saved() {
    let extra = vec![
        json!({"id": "TRINKET_A", "name": "Trinket", "type": "BATTLEGROUND_TRINKET", "dbfId": 1234}),
        json!({"id": "HERO_BASE", "name": "Base", "type": "HERO", "dbfId": 59999}),
        json!({"id": "TWIN_A", "name": "Twin A", "dbfId": 333}),
        json!({"id": "TWIN_B", "name": "Twin B", "dbfId": 333}),
    ];
    let catalog = Catalog::from_hearthstonejson(&synthetic(MIN_CARDS, extra)).unwrap();
    assert_eq!(catalog.card_of_dbf(1234), Some("TRINKET_A"));
    assert_eq!(catalog.card_of_dbf(59999), Some("HERO_BASE"));
    assert_eq!(catalog.card_of_dbf(333), None, "claimed twice: not guessed");
    assert_eq!(catalog.card_of_dbf(-1), None);
    let saved = SavedCatalog {
        catalog,
        fetched_at: 1_700_000_000,
        etag: Some("\"abc\"".into()),
    };
    let back = SavedCatalog::from_json(&saved.to_json()).unwrap();
    assert_eq!(back, saved);
    let mut edited: Value = serde_json::from_slice(&saved.to_json()).unwrap();
    edited["cards"]["7"] = json!("../EVIL");
    edited["cards"]["8"] = json!("NOT_A_CARD");
    let back = SavedCatalog::from_json(&serde_json::to_vec(&edited).unwrap()).unwrap();
    assert_eq!(
        (back.catalog.card_of_dbf(7), back.catalog.card_of_dbf(8)),
        (None, None)
    );
    // A copy from before the card links loads without them and is fetched again soon.
    let mut old: Value = serde_json::from_slice(&saved.to_json()).unwrap();
    old["format"] = json!(3);
    old.as_object_mut().unwrap().remove("cards");
    let back = SavedCatalog::from_json(&serde_json::to_vec(&old).unwrap()).unwrap();
    assert_eq!(back.catalog.card_of_dbf(1234), None);
    assert_eq!(back.catalog.hero_links().len(), 1);
    assert_eq!((back.fetched_at, back.etag), (0, None));
}

#[test]
fn hero_links_are_saved_and_checked_again_when_read() {
    let extra = vec![json!({"id": "HERO_BASE", "name": "Base", "type": "HERO", "dbfId": 59999})];
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

#[test]
fn combat_facts_are_read_saved_and_checked_again() {
    let extra = vec![
        json!({"id": "BG_SYN_1", "name": "Synth", "type": "MINION", "set": "BATTLEGROUNDS",
               "attack": 2, "health": 3, "techLevel": 2, "races": ["MECHANICAL"],
               "mechanics": ["DIVINE_SHIELD", "TRIGGER_VISUAL"]}),
        json!({"id": "CORE_SYN", "name": "Core", "type": "MINION", "set": "CORE",
               "attack": 1, "health": 1}),
    ];
    let catalog = Catalog::from_hearthstonejson(&synthetic(MIN_CARDS, extra)).unwrap();
    let facts = catalog
        .combat_facts("BG_SYN_1")
        .expect("a Battlegrounds minion");
    assert_eq!((facts.attack, facts.health, facts.tier), (2, 3, Some(2)));
    assert_eq!(facts.tribes, ["MECHANICAL"]);
    assert_eq!(facts.keywords, ["DIVINE_SHIELD"]);
    assert!(catalog.combat_facts("CORE_SYN").is_none());
    let saved = SavedCatalog {
        catalog,
        fetched_at: 5,
        etag: Some("\"e\"".into()),
    };
    let back = SavedCatalog::from_json(&saved.to_json()).unwrap();
    assert_eq!(back, saved);
    let mut edited: Value = serde_json::from_slice(&saved.to_json()).unwrap();
    edited["combat"]["BG_SYN_1"]["health"] = json!(-4);
    let back = SavedCatalog::from_json(&serde_json::to_vec(&edited).unwrap()).unwrap();
    assert!(
        back.catalog.combat_facts("BG_SYN_1").is_none(),
        "checked when read"
    );
    // A copy from before the combat facts loads without them and is fetched again soon.
    let mut old: Value = serde_json::from_slice(&saved.to_json()).unwrap();
    old["format"] = json!(4);
    old.as_object_mut().unwrap().remove("combat");
    let back = SavedCatalog::from_json(&serde_json::to_vec(&old).unwrap()).unwrap();
    assert!(back.catalog.combat_facts("BG_SYN_1").is_none());
    assert_eq!((back.fetched_at, back.etag), (0, None));
}

#[test]
fn a_skin_knows_its_base_hero_by_the_card_datas_parent_link() {
    let extra = vec![
        json!({"id": "HERO_BASE", "name": "Base", "type": "HERO", "dbfId": 59999}),
        json!({"id": "HERO_BASE_SKIN_A", "name": "Base", "type": "HERO", "dbfId": 70001,
               "battlegroundsSkinParentId": 59999}),
        // Not a hero, a parent id that is not a number, a parent of 0: left out.
        json!({"id": "MINION_X", "name": "Minion", "type": "MINION", "battlegroundsSkinParentId": 59999}),
        json!({"id": "HERO_BAD", "name": "Bad", "type": "HERO", "battlegroundsSkinParentId": "59999"}),
        json!({"id": "HERO_ZERO", "name": "Zero", "type": "HERO", "battlegroundsSkinParentId": 0}),
    ];
    let catalog = Catalog::from_hearthstonejson(&synthetic(MIN_CARDS, extra)).unwrap();
    let parents: Vec<_> = catalog
        .skin_parents()
        .iter()
        .map(|(id, d)| (id.as_str(), *d))
        .collect();
    assert_eq!(parents, [("HERO_BASE_SKIN_A", 59999)]);
    let saved = SavedCatalog {
        catalog,
        fetched_at: 1_700_000_000,
        etag: Some("\"abc\"".into()),
    };
    let back = SavedCatalog::from_json(&saved.to_json()).unwrap();
    assert_eq!(back, saved);
    let mut edited: Value = serde_json::from_slice(&saved.to_json()).unwrap();
    edited["skin_parents"]["../EVIL"] = json!(59999);
    edited["skin_parents"]["NOT_A_CARD"] = json!(59999);
    edited["skin_parents"]["HERO_BASE"] = json!(-4);
    let back = SavedCatalog::from_json(&serde_json::to_vec(&edited).unwrap()).unwrap();
    assert_eq!(back.catalog.skin_parents().len(), 1);
    // A copy from before the skin links loads without them and is fetched again soon.
    let mut old: Value = serde_json::from_slice(&saved.to_json()).unwrap();
    old["format"] = json!(5);
    old.as_object_mut().unwrap().remove("skin_parents");
    let back = SavedCatalog::from_json(&serde_json::to_vec(&old).unwrap()).unwrap();
    assert!(back.catalog.skin_parents().is_empty());
    assert_eq!(back.catalog.hero_links().len(), 2);
    assert_eq!((back.fetched_at, back.etag), (0, None));
}
