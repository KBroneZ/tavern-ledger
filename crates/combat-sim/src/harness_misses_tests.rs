use super::*;
use crate::combat::replay::{Difference, Replay, Rule};

fn miss(duos: bool, coverage: Coverage) -> Miss {
    Miss {
        game: 1,
        round: 3,
        duos,
        prediction: Prediction {
            won: 0.9,
            tied: 0.0,
            lost: 0.1,
        },
        result: CombatResult::Lost,
        coverage,
        inputs: Vec::new(),
    }
}

fn replayed(duos: bool, difference: Option<Rule>) -> Replayed {
    Replayed {
        game: 1,
        round: 3,
        duos,
        replay: Replay {
            attacks: 4,
            matched: 2,
            boards_in: 0,
            difference: difference.map(|rule| Difference {
                attack: 3,
                rule,
                detail: String::new(),
                card: None,
            }),
        },
    }
}

#[test]
fn a_miss_without_a_replay_is_told_by_its_cards() {
    let none = Coverage::default();
    let unknown = Coverage {
        not_modelled: vec!["ZZ_UNKNOWN".into()],
        approximate: Vec::new(),
    };
    let rough = Coverage {
        not_modelled: Vec::new(),
        approximate: vec![("ZZ_ROUGH".into(), "a gap")],
    };
    assert_eq!(cause_id(&miss(false, none), &[]), "no_replay_exact");
    assert_eq!(
        cause_id(&miss(false, unknown), &[]),
        "no_replay_not_modelled"
    );
    assert_eq!(cause_id(&miss(true, rough), &[]), "no_replay_approximate");
}

#[test]
fn a_replay_to_the_end_is_chance_and_a_difference_names_its_rule() {
    let m = miss(true, Coverage::default());
    assert_eq!(cause_id(&m, &[replayed(true, None)]), "chance");
    assert_eq!(
        cause_id(&m, &[replayed(true, Some(Rule::Minions))]),
        "replay_minions"
    );
    // A replay of the other mode is not this combat's.
    assert_eq!(cause_id(&m, &[replayed(false, None)]), "no_replay_exact");
}

#[test]
fn every_rule_has_its_own_id() {
    let m = miss(false, Coverage::default());
    let rules = [
        Rule::FirstAttacker,
        Rule::AfterTeammate,
        Rule::AttackOrder,
        Rule::Target,
        Rule::Minions,
        Rule::Stats,
        Rule::End,
        Rule::NoDefender,
    ];
    let mut ids: Vec<&str> = rules
        .iter()
        .map(|r| cause_id(&m, &[replayed(false, Some(*r))]))
        .collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), rules.len());
    assert!(ids.iter().all(|id| id.starts_with("replay_")));
}

#[test]
fn causes_by_id_count_per_mode_most_first_then_by_id() {
    let plain = Coverage::default();
    let misses = vec![
        miss(false, plain.clone()),
        miss(true, plain.clone()),
        miss(true, plain),
    ];
    let rows = causes_by_id(&misses, &[]);
    assert_eq!(rows, vec![("no_replay_exact", 1, 2)]);
    let rows = causes_by_id(&misses, &[replayed(true, None)]);
    // Both Duos misses are the same combat here, so both read the replay.
    assert_eq!(rows, vec![("chance", 0, 2), ("no_replay_exact", 1, 0)]);
}

#[test]
fn a_replay_that_parts_names_what_was_not_seen_then_the_cards_then_the_rule() {
    let rough = Coverage {
        not_modelled: Vec::new(),
        approximate: vec![("ZZ_ROUGH".into(), "a gap")],
    };
    let parts = [replayed(false, Some(Rule::Stats))];
    let mut m = miss(false, rough);
    assert_eq!(cause_id(&m, &parts), "approximate_card");
    m.inputs = vec![NotSeen {
        what: crate::why::Unseen::BloodGems,
        card: "ZZ_GEMS".into(),
    }];
    assert_eq!(cause_id(&m, &parts), "missing_inputs");
    assert_eq!(
        cause_id(&miss(false, Coverage::default()), &parts),
        "replay_stats"
    );
    // To the end it is chance whatever the cards.
    assert_eq!(cause_id(&m, &[replayed(false, None)]), "chance");
}
