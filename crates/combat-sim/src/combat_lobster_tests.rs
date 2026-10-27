//! Tasty Lobster (T-645): "Deathrattle: Give a random friendly Beast +2/+1.
//! Improve your future Tasty Lobsters." The user's logged replays show the
//! buff as a game-long value per player: a Lobster on the board shows it as
//! its two numbers (a golden one twice it), each Lobster Deathrattle gives
//! it (a golden one twice it) and then improves it by +2/+1, Lobsters
//! summoned in combat included.

use super::{Combat, Prepared};
use crate::model::{Minion, Tribes, Warband};
use crate::rng::Rng;

/// A Tasty Lobster showing `counters` (a golden one shows twice the buff).
fn lobster(golden: bool, counters: [i32; 2]) -> Minion {
    let (id, attack, health) = if golden {
        ("BG36_202_G", 4, 2)
    } else {
        ("BG36_202", 2, 1)
    };
    let mut m = Minion::new(id, attack, health).tribe(Tribes::BEAST).tier(3);
    m.golden = golden;
    m.counters = counters;
    m
}

/// A Beast with no effect.
fn beast(attack: i32, health: i32) -> Minion {
    Minion::new("BG21_000", attack, health)
        .tribe(Tribes::BEAST)
        .tier(1)
}

/// A non-Beast with no effect.
fn plain(attack: i32, health: i32) -> Minion {
    Minion::new("BGS_119", attack, health).tier(1)
}

fn with<R>(player: Vec<Minion>, opponent: Vec<Minion>, f: impl FnOnce(&mut Combat) -> R) -> R {
    let player = Warband::new(player);
    let opponent = Warband::new(opponent);
    let p = Prepared::new(&player, &opponent, false);
    let mut rng = Rng::new(7);
    let mut c = Combat::new(&p, &mut rng);
    c.attack_immediately(0, [true, true]);
    f(&mut c)
}

fn stats(c: &Combat, side: usize) -> Vec<(i32, i32)> {
    c.sides[side]
        .units
        .iter()
        .map(|u| (u.attack, u.health))
        .collect()
}

fn kill(c: &mut Combat, side: usize, ix: usize) {
    c.sides[side].units[ix].health = 0;
    c.process_deaths(side);
}

#[test]
fn the_buff_is_the_one_the_lobster_on_the_board_shows() {
    with(
        vec![lobster(false, [18, 9]), beast(3, 1)],
        vec![plain(0, 99)],
        |c| {
            kill(c, 0, 0);
            assert_eq!(stats(c, 0), [(21, 10)]);
        },
    );
}

#[test]
fn each_deathrattle_improves_the_buff_by_two_and_one() {
    with(
        vec![lobster(false, [18, 9]), beast(3, 1)],
        vec![plain(0, 99)],
        |c| {
            assert_eq!(c.sides[0].lobster, (18, 9));
            kill(c, 0, 0);
            assert_eq!(c.sides[0].lobster, (20, 10));
        },
    );
}

#[test]
fn a_later_lobster_gives_the_improved_buff() {
    // The second Lobster is the only Beast left when the first dies, then
    // gives the improved buff to the Beast after it.
    with(
        vec![lobster(false, [6, 3]), lobster(false, [6, 3])],
        vec![plain(0, 99)],
        |c| {
            kill(c, 0, 0);
            assert_eq!(stats(c, 0), [(8, 4)]);
        },
    );
    with(
        vec![lobster(false, [6, 3]), lobster(false, [6, 3]), beast(1, 1)],
        vec![plain(0, 99)],
        |c| {
            // Whichever Beast the first buff went to, the second gives 8/4.
            kill(c, 0, 0);
            let before: i32 = stats(c, 0).iter().map(|s| s.0).sum();
            assert_eq!(before, 2 + 1 + 6);
            kill(c, 0, 0);
            assert_eq!(c.sides[0].lobster, (10, 5));
        },
    );
}

#[test]
fn a_golden_lobster_shows_and_gives_twice_the_buff() {
    with(
        vec![lobster(true, [16, 8]), beast(1, 1)],
        vec![plain(0, 99)],
        |c| {
            assert_eq!(c.sides[0].lobster, (8, 4));
            kill(c, 0, 0);
            assert_eq!(stats(c, 0), [(17, 9)]);
            assert_eq!(c.sides[0].lobster, (10, 5));
        },
    );
}

#[test]
fn without_numbers_the_text_buff_is_used() {
    with(
        vec![lobster(false, [0, 0]), beast(1, 1)],
        vec![plain(0, 99)],
        |c| {
            assert_eq!(c.sides[0].lobster, (2, 1));
            kill(c, 0, 0);
            assert_eq!(stats(c, 0), [(3, 2)]);
        },
    );
}

#[test]
fn each_side_keeps_its_own_buff() {
    with(
        vec![lobster(false, [18, 9]), beast(1, 1)],
        vec![lobster(false, [4, 2]), beast(1, 1)],
        |c| {
            kill(c, 1, 0);
            assert_eq!(stats(c, 1), [(5, 3)]);
            assert_eq!(c.sides[0].lobster, (18, 9));
            assert_eq!(c.sides[1].lobster, (6, 3));
        },
    );
}

#[test]
fn a_non_beast_gets_nothing() {
    with(
        vec![lobster(false, [18, 9]), plain(1, 1)],
        vec![plain(0, 99)],
        |c| {
            kill(c, 0, 0);
            assert_eq!(stats(c, 0), [(1, 1)]);
            assert_eq!(c.sides[0].lobster, (20, 10));
        },
    );
}

#[test]
fn a_lobster_is_exact_only_when_a_lobster_on_the_board_shows_the_buff() {
    use crate::trials::Coverage;
    let approximate = |minions: Vec<Minion>| {
        let cov = Coverage::of(
            &Warband::new(minions),
            &Warband::new(vec![plain(1, 1)]),
            false,
        );
        cov.approximate
            .iter()
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(approximate(vec![lobster(false, [0, 0])]), ["BG36_202"]);
    assert!(approximate(vec![lobster(false, [6, 3]), lobster(true, [0, 0])]).is_empty());
    // Hoarding Hyena summons Lobsters: exact with the buff shown.
    let hyena = Minion::new("BG36_210", 3, 3).tribe(Tribes::BEAST).tier(3);
    assert_eq!(approximate(vec![hyena.clone()]), ["BG36_210"]);
    assert!(approximate(vec![hyena, lobster(false, [6, 3])]).is_empty());
}
