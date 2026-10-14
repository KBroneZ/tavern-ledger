//! Golden cards, and cards listed in one form only, on small boards whose
//! outcome is certain.

use super::{Combat, Prepared};
use crate::model::{Keywords, Minion, Tribes, Warband};
use crate::rng::Rng;
use crate::trials::{simulate, Options, Verdict};

fn band(minions: Vec<Minion>) -> Warband {
    Warband {
        minions,
        tavern_tier: Some(1),
        values: Default::default(),
    }
}

/// Crackling Cyclone's id: a card with no combat text, so the odds are not
/// refused for it.
fn plain(attack: i32, health: i32) -> Minion {
    Minion::new("BGS_119", attack, health).tier(1)
}

fn taunt(m: Minion) -> Minion {
    Minion {
        keywords: Keywords::TAUNT,
        ..m
    }
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

fn uid(c: &Combat, side: usize, at: usize) -> u32 {
    c.sides[side].units[at].uid
}

#[test]
fn a_board_with_the_two_cards_of_the_report_gets_odds() {
    // Disguised Graverobber (a Battlecry) and a golden Spark Snapper ("whenever
    // you play a Mech") do nothing in combat.
    let graverobber = Minion::new("BG28_303", 4, 4).tier(3);
    let snapper = Minion::new("BG36_851_G", 12, 16)
        .tier(5)
        .tribe(Tribes::MECH);
    for (a, b) in [
        (graverobber.clone(), snapper.clone()),
        (snapper, graverobber),
    ] {
        let options = Options {
            trials: 50,
            ..Options::default()
        };
        let verdict = simulate(&band(vec![a, b]), &band(vec![plain(1, 1)]), &options).unwrap();
        let Verdict::Odds(o) = verdict else {
            panic!("both cards are modelled: {verdict:?}");
        };
        assert_eq!(o.won, o.trials);
    }
}

#[test]
fn golden_graverobber_and_normal_spark_snapper_are_modelled_too() {
    for id in ["BG28_303_G", "BG36_851"] {
        let board = band(vec![Minion::new(id, 8, 8).tier(3)]);
        let verdict = simulate(&board, &band(vec![plain(1, 1)]), &Options::default()).unwrap();
        assert!(matches!(verdict, Verdict::Odds(_)), "{id} is not modelled");
    }
}

#[test]
fn golden_obsidian_ravager_damages_the_target_and_both_neighbours() {
    let ravager = Minion::new("BG27_017_G", 14, 14).tier(7);
    with(
        vec![ravager],
        vec![plain(0, 20), taunt(plain(0, 50)), plain(0, 20)],
        |c| {
            let r = uid(c, 0, 0);
            c.swing(0, r);
            // The target takes the Rally and the attack, the neighbours the Rally.
            assert_eq!(stats(c, 1), [(0, 6), (0, 22), (0, 6)]);
        },
    );
}

#[test]
fn normal_obsidian_ravager_still_damages_one_neighbour_only() {
    let ravager = Minion::new("BG27_017", 7, 7).tier(7);
    with(
        vec![ravager],
        vec![plain(0, 20), taunt(plain(0, 50)), plain(0, 20)],
        |c| {
            let r = uid(c, 0, 0);
            c.swing(0, r);
            let health: Vec<i32> = stats(c, 1).iter().map(|s| s.1).collect();
            assert_eq!(health[1], 36);
            assert_eq!(health.iter().filter(|h| **h == 13).count(), 1);
            assert_eq!(health.iter().filter(|h| **h == 20).count(), 1);
        },
    );
}

#[test]
fn golden_leeroy_destroys_the_minion_that_killed_it() {
    // A 12/4 golden Leeroy against a 4/30 with Divine Shield and a 0/1: the
    // 4/30 kills it (its Divine Shield stops the hit), and the Deathrattle
    // destroys it.
    let leeroy = Minion::new("BG23_318_G", 12, 4).tier(5);
    let shielded = plain(4, 30).with(Keywords::DIVINE_SHIELD);
    with(vec![leeroy], vec![shielded, plain(0, 1)], |c| {
        let killer = uid(c, 1, 0);
        c.swing(1, killer);
        assert!(c.sides[0].units.is_empty());
        let left: Vec<i32> = c.sides[1].units.iter().map(|u| u.health).collect();
        assert_eq!(left, [1], "only the 0/1 is left");
    });
}

#[test]
fn golden_goldrinn_gives_its_beasts_fourteen_attack_and_health() {
    let goldrinn = taunt(
        Minion::new("TB_BaconUps_085", 1, 1)
            .tier(5)
            .tribe(Tribes::BEAST),
    );
    let friend = plain(2, 3).tribe(Tribes::BEAST);
    with(vec![goldrinn, friend], vec![plain(5, 50)], |c| {
        let foe = uid(c, 1, 0);
        c.swing(1, foe);
        assert_eq!(stats(c, 0), [(16, 17)]);
    });
}

#[test]
fn golden_nadina_gives_six_dragons_a_divine_shield() {
    let nadina = taunt(
        Minion::new("TB_BaconUps_154", 1, 1)
            .tier(6)
            .tribe(Tribes::DRAGON),
    );
    let dragon = || plain(1, 5).tribe(Tribes::DRAGON);
    let mut board = vec![nadina];
    board.extend((0..7).map(|_| dragon()));
    with(board, vec![plain(5, 50)], |c| {
        let foe = uid(c, 1, 0);
        c.swing(1, foe);
        let shielded = c.sides[0]
            .units
            .iter()
            .filter(|u| u.kw.contains(Keywords::DIVINE_SHIELD))
            .count();
        assert_eq!(c.sides[0].units.len(), 7);
        assert_eq!(shielded, 6, "six of seven Dragons get a shield");
    });
}
