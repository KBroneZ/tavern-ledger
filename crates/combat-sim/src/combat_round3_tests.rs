//! Round 3 cards on small boards whose outcome is certain.

use super::super::{Combat, Prepared};
use crate::model::{BaseCard, Keywords, Minion, Tribes, Warband};
use crate::rng::Rng;

fn band(minions: Vec<Minion>) -> Warband {
    Warband {
        kit: None,
        minions,
        tavern_tier: Some(1),
        values: Default::default(),
    }
}

fn plain(attack: i32, health: i32) -> Minion {
    Minion::new("TEST_PLAIN", attack, health).tier(1)
}

fn with<R>(player: Vec<Minion>, opponent: Vec<Minion>, f: impl FnOnce(&mut Combat) -> R) -> R {
    let p = Prepared::new(&band(player), &band(opponent), false);
    let mut rng = Rng::new(5);
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

/// Bonehead: Deathrattle: summon two 1/1 Skeletons.
fn bonehead() -> Minion {
    Minion::new("BG28_300", 1, 1).tribe(Tribes::UNDEAD).tier(1)
}

#[test]
fn falling_sky_golem_grows_with_each_friendly_deathrattle() {
    let golem = Minion::new("BG35_342", 4, 2).tier(6);
    with(vec![bonehead(), golem], vec![plain(5, 50)], |c| {
        let b = c.sides[0].units[0].uid;
        c.swing(0, b);
        // One Deathrattle: +4/+2; two Skeletons in its place.
        assert_eq!(stats(c, 0), [(1, 1), (1, 1), (8, 4)]);
    });
}

#[test]
fn falling_sky_golem_ignores_enemy_deathrattles() {
    let golem = Minion::new("BG35_342_G", 8, 4).tier(6);
    with(vec![golem], vec![bonehead()], |c| {
        let g = c.sides[0].units[0].uid;
        c.swing(0, g);
        assert_eq!(stats(c, 0), [(8, 3)]);
    });
}

#[test]
fn plaguerunner_gives_the_undead_left_attack() {
    let runner = Minion::new("BG34_690", 4, 2).tribe(Tribes::UNDEAD).tier(4);
    // Card facts are per card id: the Undead one needs its own.
    let undead = Minion::new("TEST_UNDEAD", 2, 9)
        .tribe(Tribes::UNDEAD)
        .tier(1);
    with(vec![runner, undead, plain(2, 9)], vec![plain(5, 50)], |c| {
        let r = c.sides[0].units[0].uid;
        c.swing(0, r);
        assert_eq!(stats(c, 0), [(4, 9), (2, 9)], "Undead only");
    });
}

#[test]
fn undead_summoned_after_plaguerunner_died_get_its_attack_too() {
    // T-645: the owner's logs, Skeletons summoned after a Plaguerunner died
    // come with its Attack (this game, wherever they are).
    for (runner, more) in [("BG34_690", 2), ("BG34_690_G", 4)] {
        let r = Minion::new(runner, 4, 2).tribe(Tribes::UNDEAD).tier(4);
        with(vec![r, bonehead()], vec![plain(5, 50)], |c| {
            let first = c.sides[0].units[0].uid;
            c.swing(0, first);
            assert_eq!(stats(c, 0), [(1 + more, 1)], "{runner}: the Bonehead");
            let b = c.sides[0].units[0].uid;
            c.swing(0, b);
            assert_eq!(
                stats(c, 0),
                [(1 + more, 1), (1 + more, 1)],
                "{runner}: its Skeletons"
            );
        });
    }
}

#[test]
fn banana_slamma_doubles_a_summoned_beast_and_its_golden_triples() {
    // Two Beetles (2/2 Beasts) would be summoned by a Ravaging Scorpid; the
    // engine's own Beast token here is the Flittering Bat's 1/1.
    let bat = Minion::new("BG36_200", 1, 4).tribe(Tribes::BEAST).tier(1);
    for (slamma, attack) in [("BG26_802", 2), ("BG26_802_G", 3)] {
        let s = Minion::new(slamma, 3, 6).tribe(Tribes::BEAST).tier(4);
        with(vec![bat.clone(), s], vec![plain(0, 50)], |c| {
            let b = c.sides[0].units[0].uid;
            // Rally: summon a 1/1 Beast.
            c.swing(0, b);
            let summoned = &c.sides[0].units[1];
            assert_eq!((summoned.attack, summoned.health), (attack, 1), "{slamma}");
        });
    }
}

#[test]
fn a_reborn_beast_counts_as_summoned() {
    let mut beast = plain(3, 1).tribe(Tribes::BEAST).with(Keywords::REBORN);
    beast.base = Some(BaseCard {
        attack: 3,
        health: 1,
        keywords: Keywords::REBORN,
    });
    let leviathan = Minion::new("BG35_602", 3, 9).tribe(Tribes::BEAST).tier(5);
    with(vec![beast, leviathan], vec![plain(1, 50)], |c| {
        let b = c.sides[0].units[0].uid;
        c.swing(0, b);
        assert_eq!(
            stats(c, 0),
            [(6, 1), (3, 9)],
            "+3 Attack on the Reborn copy"
        );
    });
}

#[test]
fn a_summon_effect_skips_other_types() {
    let leviathan = Minion::new("BG35_602", 3, 9).tribe(Tribes::BEAST).tier(5);
    with(vec![bonehead(), leviathan], vec![plain(5, 50)], |c| {
        let b = c.sides[0].units[0].uid;
        c.swing(0, b);
        assert_eq!(
            stats(c, 0),
            [(1, 1), (1, 1), (3, 9)],
            "Skeletons are Undead"
        );
    });
}
