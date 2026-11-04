//! The committed conformance corpus (T-641, D-177) holds nothing private:
//! every line passes the corpus reader's checks (its keys are the ones a line
//! keeps at each level, its strings card ids or lower-case game words, its
//! numbers whole and in range, its id the hash of its content), and the raw
//! text has nothing that looks like a name, a BattleTag, an email, a time or
//! card data. The corpus is the maintainer's own games, names and times
//! stripped by `combat-corpus`.

use std::io::Read as _;
use std::path::Path;

use combat_sim::corpus::read_corpus_file;
use serde_json::Value;

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/corpus");

fn raw_lines() -> Vec<String> {
    let bytes = std::fs::read(Path::new(DIR).join("corpus.jsonl.gz")).unwrap();
    let mut text = String::new();
    flate2::read::GzDecoder::new(&bytes[..])
        .read_to_string(&mut text)
        .unwrap();
    text.lines().map(String::from).collect()
}

/// The card sets a Battlegrounds card id starts with; anything else that
/// is not a lower-case game word could be a name.
const CARD_SETS: [&str; 8] = ["BG", "BGDUO", "BGFYM", "BGS", "DMF", "EBG", "ICC", "TB"];

fn card_set_of(s: &str) -> Option<&str> {
    let set = s.split('_').next()?;
    let letters = set.trim_end_matches(|c: char| c.is_ascii_digit());
    CARD_SETS.contains(&letters).then_some(letters)
}

/// Every string in `v` is a lower-case game word, a 16-hex id, a game type
/// or a card id of a known set.
fn strings_are_known(v: &Value, path: &str) {
    match v {
        Value::Object(map) => map
            .iter()
            .for_each(|(k, v)| strings_are_known(v, &format!("{path}/{k}"))),
        Value::Array(items) => items.iter().for_each(|i| strings_are_known(i, path)),
        Value::String(s) => {
            let word = s
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b == b'_' || b.is_ascii_digit())
                && s.starts_with(|c: char| c.is_ascii_lowercase());
            let id = s.len() == 16 && s.bytes().all(|b| b.is_ascii_hexdigit());
            let game_type = s == "GT_BATTLEGROUNDS" || s == "GT_BATTLEGROUNDS_DUO";
            let card = card_set_of(s).is_some()
                && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_');
            assert!(
                word || id || game_type || card,
                "{path}: an unexpected string"
            );
        }
        _ => {}
    }
}

fn keys(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::Object(map) => {
            for (k, v) in map {
                out.push(k.clone());
                keys(v, out);
            }
        }
        Value::Array(items) => items.iter().for_each(|i| keys(i, out)),
        _ => {}
    }
}

#[test]
fn every_line_passes_the_corpus_privacy_check() {
    let corpus = read_corpus_file(&Path::new(DIR).join("corpus.jsonl.gz")).unwrap();
    assert!(corpus.len() >= 500, "{} combats", corpus.len());
    let builds: std::collections::BTreeSet<u64> = corpus.iter().map(|c| c.build).collect();
    assert_eq!(builds.into_iter().collect::<Vec<_>>(), [253_216, 254_477]);
    assert!(corpus.iter().any(|c| c.duos) && corpus.iter().any(|c| !c.duos));
}

#[test]
fn no_key_names_a_person_a_time_or_card_data() {
    const FORBIDDEN: [&str; 16] = [
        "name",
        "battletag",
        "battle_tag",
        "account",
        "account_id",
        "session",
        "email",
        "rating",
        "mmr",
        "time",
        "ms",
        "start_ms",
        "end_ms",
        "date",
        "saved_at",
        "text",
    ];
    let mut all = Vec::new();
    for line in raw_lines() {
        keys(&serde_json::from_str(&line).unwrap(), &mut all);
    }
    all.sort();
    all.dedup();
    for k in &all {
        assert!(!FORBIDDEN.contains(&k.as_str()), "key {k}");
        assert!(!k.ends_with("_ms") && !k.contains("name"), "key {k}");
    }
}

#[test]
fn the_text_has_no_battletag_email_or_free_text() {
    for (n, line) in raw_lines().iter().enumerate() {
        assert!(!line.contains('@'), "line {}: an @", n + 1);
        assert!(!line.contains('#'), "line {}: a #", n + 1);
        assert!(!line.contains(' '), "line {}: a space (free text)", n + 1);
    }
}

#[test]
fn the_card_data_each_build_is_checked_with_is_pinned() {
    let pins = std::fs::read_to_string(Path::new(DIR).join("cards.sha256")).unwrap();
    let builds: Vec<&str> = pins
        .lines()
        .map(|l| {
            let (hash, build) = l.split_once("  ").unwrap();
            assert_eq!(hash.len(), 64, "{l}");
            assert!(hash.bytes().all(|b| b.is_ascii_hexdigit()), "{l}");
            build
        })
        .collect();
    assert_eq!(builds, ["254477", "253216"]);
}

#[test]
fn every_string_is_a_game_word_an_id_or_a_card_of_a_known_set() {
    for line in raw_lines() {
        strings_are_known(&serde_json::from_str(&line).unwrap(), "");
    }
    // A name shaped like a card id does not pass.
    assert_eq!(card_set_of("John_Smith1"), None);
    assert_eq!(card_set_of("BG36_200t"), Some("BG"));
}

#[test]
fn the_baseline_holds_only_ids_outcomes_and_card_ids() {
    let text = std::fs::read_to_string(Path::new(DIR).join("baseline.json")).unwrap();
    let mut v: Value = serde_json::from_str(&text).unwrap();
    // The engine's version is the one string that is not a word or an id.
    let engine = v.as_object_mut().unwrap().remove("engine").unwrap();
    let version = engine.as_str().unwrap();
    assert!(
        version.split('.').all(|p| p.parse::<u32>().is_ok()),
        "engine {version}"
    );
    strings_are_known(&v, "");
    let mut all = Vec::new();
    keys(&v, &mut all);
    for k in all {
        let combat_id = k.len() == 16 && k.bytes().all(|b| b.is_ascii_hexdigit());
        let word = k.bytes().all(|b| b.is_ascii_lowercase() || b == b'_');
        assert!(combat_id || word, "baseline key {k}");
    }
}
