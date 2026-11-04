//! The conformance check end to end on a small synthetic corpus (T-642):
//! read the corpus, play it with each build's card data and compare it
//! with the committed baseline, as `combat-conformance` does. A change to
//! the engine that moves one of these combats fails here; if it is meant,
//! rewrite the baseline with `CONFORMANCE_BLESS=1 cargo test -p combat-sim
//! --test conformance` and commit it.

use std::collections::BTreeMap;
use std::path::Path;

use combat_sim::conformance::{baseline_json, compare, read_baseline, render_diff, run};
use combat_sim::corpus::read_corpus_file;
use combat_sim::harness::load_cards;

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/conformance");

#[test]
fn the_synthetic_corpus_plays_as_in_the_baseline() {
    let corpus = read_corpus_file(&Path::new(DIR).join("corpus.jsonl")).unwrap();
    let db =
        load_cards(&std::fs::read_to_string(Path::new(DIR).join("cards.json")).unwrap()).unwrap();
    let cards = BTreeMap::from([(253_216, db.clone()), (254_477, db)]);
    let baseline_path = Path::new(DIR).join("baseline.json");
    let (options, baseline) =
        read_baseline(&std::fs::read_to_string(&baseline_path).unwrap()).unwrap();
    let now = run(&corpus, &cards, &options).unwrap();
    if std::env::var_os("CONFORMANCE_BLESS").is_some() {
        std::fs::write(&baseline_path, baseline_json(&now, &options)).unwrap();
        return;
    }
    let diff = compare(&baseline, &now);
    assert!(!diff.fails(), "the engine moved:\n{}", render_diff(&diff));
    // The corpus covers a replay that follows the game, one that leaves it
    // at the first attack, a combat without a replay and a Duos combat.
    assert_eq!(now.len(), 4);
    let diverging: Vec<_> = now
        .values()
        .filter_map(|o| o.replay.as_ref()?.diverge.as_ref())
        .collect();
    assert_eq!(diverging.len(), 1);
    assert_eq!(diverging[0].rule, "replay_first_attacker");
}
