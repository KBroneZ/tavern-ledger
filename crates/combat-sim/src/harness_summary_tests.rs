//! The summary on the synthetic history: counts only, no game or round.

use super::*;
use crate::harness::{load_cards, load_history, run};
use crate::harness_export::accuracy;
use crate::harness_logs::replay_all;
use crate::harness_misses::{duos_misses, solo_misses};

const CARDS: &str = include_str!("../tests/data/cards.json");
const GAMES: &str = include_str!("../tests/data/games.jsonl");

#[test]
fn the_summary_has_counts_and_no_game_or_round() {
    let db = load_cards(CARDS).unwrap();
    let history = load_history(GAMES, &db);
    let options = Options {
        trials: 100,
        ..Options::default()
    };
    let run = run(&history, &options);
    let rows = replay_all(&history.saved, options.seed);
    let mut misses = solo_misses(&run.scored);
    misses.extend(duos_misses(&history.duos, &options));
    let acc = accuracy(&history.combats, &history.duos, &options);
    let text = render_summary(&Summary {
        sources: 1,
        history: &history,
        run: &run,
        accuracy: &acc,
        replays: &rows,
        unreadable: history.saved_unreadable,
        misses: &misses,
        options: &options,
    });
    assert!(
        text.contains(&format!("Games: {}", history.games)),
        "{text}"
    );
    assert!(
        text.contains("| Solo |") && text.contains("| Duos |"),
        "{text}"
    );
    assert!(text.contains("## Misses by cause"), "{text}");
    assert!(
        !text.contains("game 1") && !text.contains("round 1"),
        "{text}"
    );
}
