//! Humming Bird (T-645): "For the rest of this combat, your Beasts have +1
//! Attack" reaches the Beasts summoned after the start of combat too, as the
//! user's logged replays show (a Foraging Bat summoned as a 2/1 beside one
//! Humming Bird, five combats of one game).

use super::{Combat, Prepared};
use crate::model::{Minion, Tribes, Warband};
use crate::rng::Rng;

fn plain(attack: i32, health: i32) -> Minion {
    Minion::new("BGS_119", attack, health).tier(1)
}

fn bird() -> Minion {
    Minion::new("BG26_805", 1, 4).tribe(Tribes::BEAST).tier(2)
}

fn golden_bird() -> Minion {
    Minion::new("BG26_805_G", 2, 8).tribe(Tribes::BEAST).tier(2)
}

/// Buzzing Vermin: a Beast whose Deathrattle summons a 2/2 Beetle.
fn vermin() -> Minion {
    Minion::new("BG31_803", 1, 1).tribe(Tribes::BEAST).tier(1)
}

/// A non-Beast whose Deathrattle summons two 1/1 Undead Skeletons.
fn skeletons() -> Minion {
    Minion::new("BG28_300", 1, 1).tribe(Tribes::UNDEAD).tier(1)
}

/// Prepares the combat; with `soc` it plays the start of combat, else it
/// opens a board taken after it. Then `f` acts on it.
fn with<R>(player: Vec<Minion>, soc: bool, f: impl FnOnce(&mut Combat) -> R) -> R {
    let player = Warband::new(player);
    let opponent = Warband::new(vec![plain(0, 99)]);
    let p = Prepared::new(&player, &opponent, soc);
    let mut rng = Rng::new(7);
    let mut c = Combat::new(&p, &mut rng);
    if soc {
        c.start_of_combat(0, [true, true]);
    } else {
        c.attack_immediately(0, [true, true]);
    }
    f(&mut c)
}

fn stats(c: &Combat, side: usize) -> Vec<(i32, i32)> {
    c.sides[side]
        .units
        .iter()
        .map(|u| (u.attack, u.health))
        .collect()
}

/// The friendly minion at `ix` dies, and its death is resolved.
fn kill(c: &mut Combat, ix: usize) {
    c.sides[0].units[ix].health = 0;
    c.process_deaths(0);
}

#[test]
fn start_of_combat_gives_the_beasts_on_the_board_one_attack() {
    with(vec![bird(), vermin(), plain(1, 1)], true, |c| {
        assert_eq!(stats(c, 0), [(2, 4), (2, 1), (1, 1)]);
    });
}

#[test]
fn a_beast_summoned_later_has_the_attack_too() {
    with(vec![bird(), vermin()], true, |c| {
        kill(c, 1);
        assert_eq!(stats(c, 0), [(2, 4), (3, 2)]);
    });
}

#[test]
fn on_a_board_taken_after_start_of_combat_only_later_beasts_get_it() {
    // The board already shows the Attack; the Beetle comes in after it.
    with(vec![bird(), vermin()], false, |c| {
        assert_eq!(stats(c, 0), [(1, 4), (1, 1)]);
        kill(c, 1);
        assert_eq!(stats(c, 0), [(1, 4), (3, 2)]);
    });
}

#[test]
fn it_lasts_after_the_bird_dies_and_adds_up() {
    with(vec![bird(), golden_bird(), vermin()], false, |c| {
        kill(c, 0);
        kill(c, 0);
        kill(c, 0);
        assert_eq!(stats(c, 0), [(5, 2)]);
    });
}

#[test]
fn other_minion_types_are_left_alone() {
    with(vec![bird(), skeletons()], false, |c| {
        kill(c, 1);
        assert_eq!(stats(c, 0), [(1, 4), (1, 1), (1, 1)]);
    });
}

#[test]
fn the_other_side_gets_nothing() {
    let player = Warband::new(vec![bird()]);
    let opponent = Warband::new(vec![vermin()]);
    let p = Prepared::new(&player, &opponent, false);
    let mut rng = Rng::new(7);
    let mut c = Combat::new(&p, &mut rng);
    c.attack_immediately(0, [true, true]);
    c.sides[1].units[0].health = 0;
    c.process_deaths(1);
    assert_eq!(stats(&c, 1), [(2, 2)]);
}
