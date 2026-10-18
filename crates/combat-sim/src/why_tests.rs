//! Each rule of D-127 on hand-made boards: a 3/3 that always beats a 1/1,
//! told it lost, with a log that does (or does not) follow the engine.

use super::*;
use crate::model::{GameValues, Minion, Warband};

/// A card with no combat text (`cards::NO_COMBAT_EFFECT`).
const PLAIN: &str = "BG28_603t";
/// Blood Gem cards and a hand summoner (approximate for what is not seen),
/// and a card approximate for a rule (which minion it picks).
const GEMS: &str = "BG33_886";
const HAND: &str = crate::cards::HAND_SUMMONERS[0];
const PICKS: &str = "BG28_309";

fn plain(attack: i32, health: i32) -> Minion {
    Minion::new(PLAIN, attack, health).tier(1)
}

fn team(minions: Vec<Minion>, values: GameValues) -> Team {
    Team::solo(Warband {
        kit: None,
        minions,
        tavern_tier: Some(1),
        values,
    })
}

/// The player's 3/3 (and `extra`) against a 1/1.
fn boards(extra: Option<&str>, values: GameValues) -> (Team, Team) {
    let mut own = vec![plain(3, 3)];
    own.extend(extra.map(|id| Minion::new(id, 1, 1).tier(1)));
    (
        team(own, values),
        team(vec![plain(1, 1)], GameValues::default()),
    )
}

/// A log where the opponent's 1/1 attacks first, which the engine does not
/// allow (the player has as many minions or more).
fn opponent_first(player: &Team, opponent: &Team) -> Vec<LoggedStep> {
    vec![LoggedStep {
        attacker: (1, 0),
        defender: Some(0),
        before: [player.first.minions.clone(), opponent.first.minions.clone()],
        all_keywords: true,
    }]
}

fn options() -> Options {
    Options {
        trials: 200,
        ..Options::default()
    }
}

fn explain_with(
    player: &Team,
    opponent: &Team,
    result: CombatResult,
    replay: Replayable,
    gifts: &[String],
) -> Explained {
    explain(
        &CombatFacts {
            player,
            opponent,
            result,
            replay,
            unknown_gifts: gifts,
        },
        &options(),
    )
}

fn cause_of(extra: Option<&str>, values: GameValues, gifts: &[String]) -> Cause {
    let (p, o) = boards(extra, values);
    let steps = opponent_first(&p, &o);
    let e = explain_with(&p, &o, CombatResult::Lost, Replayable::Steps(&steps), gifts);
    e.cause.expect("odds")
}

#[test]
fn the_favourite_happening_is_matched_with_the_odds() {
    let (p, o) = boards(None, GameValues::default());
    let e = explain_with(&p, &o, CombatResult::Won, Replayable::NotSaved, &[]);
    assert_eq!(e.cause, Some(Cause::Matched));
    let Said::Odds(odds) = e.said else {
        panic!("{e:?}")
    };
    assert_eq!((odds.won, odds.lost), (1.0, 0.0));
}

#[test]
fn a_miss_without_a_replay_says_so_and_guesses_nothing() {
    let (p, o) = boards(Some(GEMS), GameValues::default());
    for (replay, why) in [
        (Replayable::NotSaved, NoReplay::NotSaved),
        (Replayable::Unreadable, NoReplay::Unreadable),
        (Replayable::Steps(&[]), NoReplay::Unreadable),
    ] {
        let e = explain_with(&p, &o, CombatResult::Lost, replay, &[]);
        let cards = Cards::Approximate;
        assert_eq!(e.cause, Some(Cause::NoReplay { why, cards }));
        assert_eq!(e.cause.unwrap().id(), "no_replay_approximate");
    }
}

#[test]
fn a_replay_that_follows_the_game_to_the_end_is_chance() {
    // The player's 3/3 and a 1/1 against a 1/2 (more minions: the player
    // first): the 3/3 kills it, as the log has it.
    let p = team(vec![plain(3, 3), plain(1, 1)], GameValues::default());
    let o = team(vec![plain(1, 2)], GameValues::default());
    let steps = opponent_first(&p, &o);
    let steps = vec![LoggedStep {
        attacker: (0, 0),
        ..steps[0].clone()
    }];
    let c = CombatFacts {
        player: &p,
        opponent: &o,
        result: CombatResult::Won,
        replay: Replayable::Steps(&steps),
        unknown_gifts: &[],
    };
    // As if the odds had said otherwise: only the replay decides.
    assert_eq!(
        missed(&c, &Coverage::default(), 1),
        Cause::Chance { attacks: 1 }
    );
}

#[test]
fn a_replay_that_stops_early_is_no_replay() {
    let (p, o) = boards(None, GameValues::default());
    let mut steps = opponent_first(&p, &o);
    steps[0].attacker = (0, 0);
    steps[0].defender = None;
    let e = explain_with(&p, &o, CombatResult::Lost, Replayable::Steps(&steps), &[]);
    let cause = e.cause.expect("odds");
    assert!(matches!(
        cause,
        Cause::NoReplay {
            why: NoReplay::Stops,
            ..
        }
    ));
    assert_eq!(cause.id(), "replay_no_defender");
}

#[test]
fn exact_boards_whose_replay_parts_are_a_rule_difference_naming_the_step() {
    let Cause::RuleDifference(d) = cause_of(Some(PLAIN), GameValues::default(), &[]) else {
        panic!("a rule difference")
    };
    assert_eq!((d.attack, d.rule), (1, Rule::FirstAttacker));
    assert_eq!(d.card.as_deref(), Some(PLAIN));
}

#[test]
fn blood_gems_without_their_upgrades_are_missing_inputs() {
    let Cause::MissingInputs { inputs, parted } = cause_of(Some(GEMS), GameValues::default(), &[])
    else {
        panic!("missing inputs")
    };
    assert_eq!(
        inputs,
        vec![NotSeen {
            what: Unseen::BloodGems,
            card: GEMS.into()
        }]
    );
    assert_eq!(parted.rule, Rule::FirstAttacker);
    // With the upgrades the board brings, the card is exact again.
    let known = GameValues {
        blood_gem: (1, 1),
        ..GameValues::default()
    };
    assert!(matches!(
        cause_of(Some(GEMS), known, &[]),
        Cause::RuleDifference(_)
    ));
}

#[test]
fn a_hand_summon_and_an_unnamed_gift_are_missing_inputs() {
    let Cause::MissingInputs { inputs, .. } = cause_of(Some(HAND), GameValues::default(), &[])
    else {
        panic!("missing inputs")
    };
    assert_eq!(inputs[0].what, Unseen::Hand);
    let gifted = [PLAIN.to_string()];
    let Cause::MissingInputs { inputs, .. } = cause_of(Some(PLAIN), GameValues::default(), &gifted)
    else {
        panic!("missing inputs")
    };
    assert_eq!(
        inputs,
        vec![NotSeen {
            what: Unseen::Gift,
            card: PLAIN.into()
        }]
    );
}

#[test]
fn a_card_approximate_for_a_rule_is_an_approximate_card() {
    let Cause::ApproximateCard { cards, parted } =
        cause_of(Some(PICKS), GameValues::default(), &[])
    else {
        panic!("an approximate card")
    };
    assert_eq!(cards, vec![PICKS.to_string()]);
    assert_eq!(parted.attack, 1);
}

#[test]
fn a_card_the_engine_does_not_know_gives_no_odds_and_no_cause() {
    let (p, o) = boards(Some("TEST_UNKNOWN"), GameValues::default());
    let e = explain_with(&p, &o, CombatResult::Lost, Replayable::NotSaved, &[]);
    assert_eq!(e.said, Said::NotModelled(vec!["TEST_UNKNOWN".into()]));
    assert_eq!(e.cause, None);
}

#[test]
fn every_cause_has_an_id_from_the_shared_list() {
    assert_eq!(Cause::Matched.id(), "matched");
    assert_eq!(Cause::Chance { attacks: 3 }.id(), "chance");
    let exact = Cause::NoReplay {
        why: NoReplay::NotSaved,
        cards: Cards::Exact,
    };
    assert_eq!(exact.id(), "no_replay_exact");
    let cause = cause_of(Some(PLAIN), GameValues::default(), &[]);
    assert_eq!(cause.id(), "replay_first_attacker");
    let mut ids: Vec<&str> = [
        Rule::FirstAttacker,
        Rule::AfterTeammate,
        Rule::AttackOrder,
        Rule::Target,
        Rule::Minions,
        Rule::Stats,
        Rule::End,
        Rule::NoDefender,
    ]
    .into_iter()
    .map(rule_id)
    .collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 8);
    assert!(ids.iter().all(|id| id.starts_with("replay_")));
}

#[test]
fn a_game_long_value_is_not_seen_only_while_its_board_has_none() {
    let none = GameValues::default();
    let all = GameValues {
        blood_gem: (1, 1),
        spell: (1, 1),
        knights_dead: 2,
        undead_attack: 1,
        beetle: (1, 1),
    };
    let cases = [
        (r1::BLOOD_GEM, Unseen::BloodGems),
        (r2::SPELL, Unseen::Spells),
        (r2::KNIGHTS, Unseen::Knights),
        (r4::BEETLE_BUFFS, Unseen::Beetles),
    ];
    for (why, what) in cases {
        assert_eq!(unseen_of(why, &none), Some(what), "{why}");
        assert_eq!(unseen_of(why, &all), None, "{why}");
    }
    // A hand summon is never seen; a rule gap is not about what is seen.
    assert_eq!(unseen_of(r5::FROM_HAND, &all), Some(Unseen::Hand));
    assert_eq!(unseen_of("which minions it picks", &none), None);
}

#[test]
fn missing_inputs_and_approximate_cards_have_their_own_ids() {
    let gems = cause_of(Some(GEMS), GameValues::default(), &[]);
    assert_eq!(gems.id(), "missing_inputs");
    assert_eq!(
        cause_of(Some(PICKS), GameValues::default(), &[]).id(),
        "approximate_card"
    );
}
