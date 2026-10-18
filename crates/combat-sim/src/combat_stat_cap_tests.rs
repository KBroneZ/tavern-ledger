//! Stats at the 32-bit limit (T-621): the website's simulator takes Attack
//! and Health up to `i32::MAX`. The engine saturates there, with no wrap and
//! no panic; what the game does at the limit is not confirmed by an official
//! source (the cap is a community report), so saturating is the stated choice.

use super::{Combat, Prepared};
use crate::combat::CombatResult;
use crate::model::{GameValues, Keywords, Minion, Tribes, Warband};
use crate::rng::Rng;
use crate::trials::{simulate, Options, Verdict};

const MAX: i32 = i32::MAX;

fn band(minions: Vec<Minion>) -> Warband {
    Warband {
        kit: None,
        minions,
        tavern_tier: Some(1),
        values: Default::default(),
    }
}

fn plain(attack: i32, health: i32) -> Minion {
    Minion::new("BGS_119", attack, health).tier(1)
}

fn with<R>(player: Warband, opponent: Vec<Minion>, f: impl FnOnce(&mut Combat) -> R) -> R {
    let p = Prepared::new(&player, &band(opponent), false);
    let mut rng = Rng::new(7);
    let mut c = Combat::new(&p, &mut rng);
    f(&mut c)
}

fn stats(c: &Combat, side: usize) -> Vec<(i32, i32)> {
    c.sides[side]
        .units
        .iter()
        .map(|u| (u.attack, u.health))
        .collect()
}

fn only_result(player: Vec<Minion>, opponent: Vec<Minion>) -> CombatResult {
    let options = Options {
        trials: 50,
        ..Options::default()
    };
    let Ok(Verdict::Odds(o)) = simulate(&band(player), &band(opponent), &options) else {
        panic!("odds expected");
    };
    assert_eq!(o.won + o.tied + o.lost, o.trials);
    match (o.won, o.tied, o.lost) {
        (w, 0, 0) if w == o.trials => CombatResult::Won,
        (0, t, 0) if t == o.trials => CombatResult::Tie,
        (0, 0, l) if l == o.trials => CombatResult::Lost,
        other => panic!("one result expected: {other:?}"),
    }
}

#[test]
fn a_buff_past_the_limit_stops_at_it() {
    with(
        band(vec![plain(MAX - 5, MAX - 5)]),
        vec![plain(0, 9)],
        |c| {
            let uid = c.sides[0].units[0].uid;
            c.buff(0, uid, 10, 10);
            assert_eq!(stats(c, 0), vec![(MAX, MAX)]);
            c.buff(0, uid, MAX, MAX);
            assert_eq!(stats(c, 0), vec![(MAX, MAX)]);
        },
    );
}

#[test]
fn stats_that_double_stop_at_the_limit() {
    // A Dark Gift that multiplies a minion's stats.
    with(
        band(vec![plain(MAX / 2 + 1, MAX / 2 + 1)]),
        vec![plain(0, 9)],
        |c| {
            let uid = c.sides[0].units[0].uid;
            c.multiply_stats(0, uid, 3, 3);
            assert_eq!(stats(c, 0), vec![(MAX, MAX)]);
        },
    );
}

#[test]
fn damage_takes_a_huge_health_down_without_wrapping() {
    with(band(vec![plain(1, MAX)]), vec![plain(MAX, MAX)], |c| {
        let (uid, by) = (c.sides[0].units[0].uid, c.sides[1].units[0].uid);
        c.damage(0, uid, MAX - 1, Keywords::NONE, by);
        assert_eq!(stats(c, 0), vec![(1, 1)]);
        c.damage(0, uid, MAX, Keywords::NONE, by);
        assert!(!c.sides[0].units[0].alive());
    });
}

#[test]
fn a_summon_with_game_long_attack_past_the_limit_stops_at_it() {
    // Undead have the most Attack a number holds this game; Bonehead's
    // Skeletons (1/1 Undead) come in at the limit, not wrapped negative.
    let bonehead = Minion::new("BG28_300", 1, 1).tribe(Tribes::UNDEAD).tier(1);
    let player = Warband {
        values: GameValues {
            undead_attack: MAX,
            ..GameValues::default()
        },
        ..band(vec![bonehead])
    };
    with(player, vec![plain(0, 9)], |c| {
        let by = c.sides[1].units[0].uid;
        let uid = c.sides[0].units[0].uid;
        c.damage(0, uid, 1, Keywords::NONE, by);
        c.process_deaths(1);
        assert_eq!(stats(c, 0), vec![(MAX, 1), (MAX, 1)]);
    });
}

#[test]
fn a_minion_that_comes_back_with_game_long_attack_past_the_limit_stops_at_it() {
    let mut reborn = Minion::new("BG28_300", 1, 1)
        .tribe(Tribes::UNDEAD)
        .with(Keywords::REBORN)
        .tier(1);
    reborn.base = Some(crate::model::BaseCard {
        attack: 2,
        health: 2,
        keywords: Keywords::NONE,
    });
    let player = Warband {
        values: GameValues {
            undead_attack: MAX,
            ..GameValues::default()
        },
        ..band(vec![reborn])
    };
    with(player, vec![plain(0, 9)], |c| {
        let (uid, by) = (c.sides[0].units[0].uid, c.sides[1].units[0].uid);
        c.damage(0, uid, 1, Keywords::NONE, by);
        c.process_deaths(1);
        let back = c.sides[0].units.last().expect("the Reborn copy").attack;
        assert_eq!(back, MAX);
    });
}

#[test]
fn whole_combats_at_the_edges_end_with_the_right_result() {
    for stat in [99_999, 100_000, 1_000_000_000, MAX - 1, MAX] {
        // The big minion kills a small one and survives its hit back.
        let big = || plain(stat, stat);
        assert_eq!(
            only_result(vec![big()], vec![plain(1, 1)]),
            CombatResult::Won
        );
        assert_eq!(
            only_result(vec![plain(1, 1)], vec![big()]),
            CombatResult::Lost
        );
        // Equal minions kill each other.
        assert_eq!(only_result(vec![big()], vec![big()]), CombatResult::Tie);
    }
}

#[test]
fn a_hit_of_the_limit_kills_a_minion_with_the_limit_of_health() {
    assert_eq!(
        only_result(vec![plain(MAX, 1)], vec![plain(1, MAX)]),
        CombatResult::Tie,
        "both die"
    );
    assert_eq!(
        only_result(vec![plain(MAX, MAX)], vec![plain(MAX - 1, MAX - 1)]),
        CombatResult::Won
    );
}
