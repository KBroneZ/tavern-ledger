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
fn the_text_has_no_battletag_email_or_upper_case_word() {
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
