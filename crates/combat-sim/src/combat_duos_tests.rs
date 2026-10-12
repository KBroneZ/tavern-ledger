//! Duos combats (T-608): two boards per team, the second coming in once the
//! first is gone. Rules from the user's logged Duos combats, section 3 of
//! `docs/research/combat-simulator.md`.

use super::*;
use crate::model::{Minion, Team};
use crate::trials::{simulate, simulate_teams, Odds, Options, Verdict};

const PLAIN: &str = "TEST_PLAIN";

fn plain(attack: i32, health: i32) -> Minion {
    Minion::new(PLAIN, attack, health).tier(1)
}

fn band(tier: u8, minions: Vec<Minion>) -> Warband {
    Warband {
        minions,
        tavern_tier: Some(tier),
        values: Default::default(),
    }
}

fn teams(player: Team, opponent: Team) -> Odds {
    let opts = Options {
        trials: 2_000,
        seed: 11,
        start_of_combat: false,
        allow_unmodelled: true,
    };
    match simulate_teams(&player, &opponent, &opts).unwrap() {
        Verdict::Odds(o) => o,
        Verdict::NotModelled(c) => panic!("not modelled: {c:?}"),
    }
}

#[test]
fn the_teammates_board_comes_in_when_the_first_is_gone() {
    let player = Team::duo(band(1, vec![plain(1, 1)]), band(4, vec![plain(5, 5)]));
    let opponent = Team::solo(band(1, vec![plain(2, 2)]));
    let o = teams(player, opponent);
    assert_eq!(o.won, o.trials, "{o:?}");
    // The hero whose board ends the combat (tier 4) and its one minion.
    assert_eq!(o.damage_dealt.get(&5), Some(&o.trials));
}

#[test]
fn a_teammate_board_that_never_fought_counts_in_the_damage() {
    let player = Team::duo(
        band(1, vec![plain(5, 5)]),
        band(3, vec![plain(1, 1), plain(1, 1)]),
    );
    let opponent = Team::solo(band(1, vec![plain(1, 1)]));
    let o = teams(player, opponent);
    assert_eq!(o.won, o.trials);
    // Tier 1 hero of the board on the field + its survivor + both waiting minions.
    assert_eq!(o.damage_dealt.get(&4), Some(&o.trials));
}

#[test]
fn a_team_loses_only_when_both_boards_are_gone() {
    let player = Team::duo(band(1, vec![plain(1, 1)]), band(1, vec![plain(1, 1)]));
    let opponent = Team::solo(band(1, vec![plain(1, 3)]));
    let o = teams(player, opponent);
    // 1/3 takes two hits of 1 and kills both 1/1s: it survives on 1 Health.
    assert_eq!(o.lost, o.trials, "{o:?}");
    assert_eq!(o.damage_taken.get(&2), Some(&o.trials));
}

#[test]
fn two_trades_in_a_row_tie() {
    let player = Team::duo(band(1, vec![plain(1, 1)]), band(1, vec![plain(1, 1)]));
    let opponent = Team::duo(band(1, vec![plain(1, 1)]), band(1, vec![plain(1, 1)]));
    let o = teams(player, opponent);
    assert_eq!(o.tied, o.trials);
}

#[test]
fn one_board_against_both_of_theirs_when_they_die_together() {
    // Both first boards trade; both second boards come in at once.
    let player = Team::duo(band(1, vec![plain(1, 1)]), band(2, vec![plain(3, 3)]));
    let opponent = Team::duo(band(1, vec![plain(1, 1)]), band(1, vec![plain(1, 1)]));
    let o = teams(player, opponent);
    assert_eq!(o.won, o.trials);
    assert_eq!(o.damage_dealt.get(&3), Some(&o.trials));
}

#[test]
fn teams_without_a_second_board_play_as_in_solo() {
    let p = band(2, vec![plain(3, 4), plain(2, 2)]);
    let q = band(3, vec![plain(2, 3), plain(4, 1)]);
    let opts = Options {
        trials: 3_000,
        seed: 5,
        start_of_combat: false,
        allow_unmodelled: true,
    };
    let solo = simulate(&p, &q, &opts).unwrap();
    let duo = simulate_teams(&Team::solo(p), &Team::solo(q), &opts).unwrap();
    assert_eq!(solo, duo);
}

#[test]
fn an_unknown_card_on_the_waiting_board_gives_no_number() {
    let player = Team::duo(
        band(1, vec![plain(1, 1)]),
        band(1, vec![Minion::new("NOPE", 1, 1)]),
    );
    let opponent = Team::solo(band(1, vec![plain(1, 1)]));
    let opts = Options {
        trials: 10,
        ..Options::default()
    };
    match simulate_teams(&player, &opponent, &opts).unwrap() {
        Verdict::NotModelled(c) => assert!(c.not_modelled.contains(&"NOPE".to_string())),
        other => panic!("{other:?}"),
    }
}

#[test]
fn an_impossible_waiting_board_is_refused() {
    let player = Team::duo(band(1, vec![plain(1, 1)]), band(1, vec![plain(1, 0)]));
    let opponent = Team::solo(band(1, vec![plain(1, 1)]));
    assert!(simulate_teams(&player, &opponent, &Options::default()).is_err());
}

#[test]
fn the_new_board_starts_left_and_the_other_side_keeps_its_place() {
    let player = Team::duo(
        band(1, vec![plain(1, 1)]),
        band(1, vec![plain(1, 9), plain(1, 9), plain(1, 9)]),
    );
    let opponent = Team::solo(band(1, vec![plain(1, 9), plain(1, 9)]));
    let p = Prepared::teams(&player, &opponent, false);
    let mut rng = Rng::new(3);
    let mut c = Combat::new(&p, &mut rng);
    c.sides[0].units.clear();
    c.sides[0].next = 1;
    c.sides[1].next = 1;
    // Three minions against two: the incoming side attacks first.
    assert_eq!(c.bring_in_teammates(), Some(0));
    assert_eq!(c.sides[0].units.len(), 3);
    assert_eq!(c.sides[0].next, 0);
    assert_eq!(c.sides[1].next, 1);
    // Fresh ids: nothing summoned later can share one with the new board.
    let fresh = c.fresh(0, 1, 1, Keywords::NONE).uid;
    assert!(c.sides[0].units.iter().all(|u| u.uid < fresh));
    // Only once.
    c.sides[0].units.clear();
    assert_eq!(c.bring_in_teammates(), None);
    assert!(c.finished());
}

#[test]
fn fewer_minions_coming_in_lets_the_other_side_attack_first() {
    let player = Team::duo(band(1, vec![plain(1, 1)]), band(1, vec![plain(1, 9)]));
    let opponent = Team::solo(band(1, vec![plain(1, 9), plain(1, 9)]));
    let p = Prepared::teams(&player, &opponent, false);
    let mut rng = Rng::new(3);
    let mut c = Combat::new(&p, &mut rng);
    c.sides[0].units.clear();
    assert_eq!(c.bring_in_teammates(), Some(1));
}
