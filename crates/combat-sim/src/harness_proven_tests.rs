use super::*;
use crate::combat::replay::{Difference, LoggedStep, Replay, Rule};
use crate::model::{Minion, Team, Warband};

fn board(ids: &[&str]) -> Vec<Minion> {
    ids.iter().map(|id| Minion::new(*id, 1, 1)).collect()
}

fn team(ids: &[&str]) -> Team {
    Team::solo(Warband::new(board(ids)))
}

/// A combat of `game` with these boards, and one attack whose boards
/// before it are `later` (a minion summoned since the start shows there).
fn combat(game: usize, player: &[&str], opponent: &[&str], later: &[&str]) -> LogCombat {
    LogCombat {
        game,
        round: 1,
        duos: false,
        player: team(player),
        opponent: team(opponent),
        steps: vec![LoggedStep {
            attacker: (0, 0),
            defender: Some(0),
            before: [board(later), board(opponent)],
            all_keywords: false,
        }],
        result: None,
    }
}

fn replayed(game: usize, followed: bool) -> Replayed {
    Replayed {
        game,
        round: 1,
        duos: false,
        replay: Replay {
            attacks: 1,
            matched: usize::from(followed),
            difference: (!followed).then_some(Difference {
                attack: 1,
                rule: Rule::Stats,
                detail: String::new(),
                card: None,
            }),
            boards_in: 0,
            rerolled: 0,
        },
    }
}

#[test]
fn a_replay_followed_to_the_end_proves_the_modelled_cards_on_its_boards() {
    let saved = [combat(
        1,
        &["BGS_119", "ZZ_UNKNOWN"],
        &["BG_ICC_026t"],
        &["BGS_119"],
    )];
    let proven = proven_cards(&saved, &[replayed(1, true)], &[Some(254_477)]);

    let ids: Vec<&str> = proven[&254_477].iter().map(String::as_str).collect();
    assert_eq!(ids, ["BGS_119", "BG_ICC_026t"]);
}

#[test]
fn a_minion_summoned_during_the_combat_counts_too() {
    let saved = [combat(
        1,
        &["BGS_119"],
        &["BGS_119"],
        &["BGS_119", "BG_ICC_026t"],
    )];
    let proven = proven_cards(&saved, &[replayed(1, true)], &[Some(254_477)]);

    assert!(proven[&254_477].contains("BG_ICC_026t"));
}

#[test]
fn a_replay_that_leaves_the_game_proves_nothing() {
    let saved = [combat(1, &["BGS_119"], &["BG_ICC_026t"], &["BGS_119"])];
    let proven = proven_cards(&saved, &[replayed(1, false)], &[Some(254_477)]);

    assert!(proven.is_empty());
}

#[test]
fn a_game_without_a_build_proves_nothing() {
    let saved = [combat(2, &["BGS_119"], &["BG_ICC_026t"], &["BGS_119"])];
    let proven = proven_cards(&saved, &[replayed(2, true)], &[Some(254_477), None]);

    assert!(proven.is_empty());
}

#[test]
fn each_build_keeps_its_own_cards() {
    let saved = [
        combat(1, &["BGS_119"], &["BGS_119"], &["BGS_119"]),
        combat(2, &["BG_ICC_026t"], &["BG_ICC_026t"], &["BG_ICC_026t"]),
    ];
    let replays = [replayed(1, true), replayed(2, true)];
    let proven = proven_cards(&saved, &replays, &[Some(253_216), Some(254_477)]);

    assert_eq!(proven[&253_216].len(), 1);
    assert!(proven[&254_477].contains("BG_ICC_026t"));
    assert!(!proven[&254_477].contains("BGS_119"));
}

#[test]
fn text_only_is_every_modelled_card_not_proven() {
    let modelled = crate::cards::modelled_ids().len();
    let proven: BTreeSet<String> = ["BGS_119".to_string()].into();

    assert!(modelled > 100);
    assert_eq!(text_only(&proven), modelled - 1);
}
