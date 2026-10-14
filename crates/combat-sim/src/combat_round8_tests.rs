//! Card round 8 (T-616): the cards that act in combat, from their card
//! texts, on small boards whose outcome is certain.

use super::{Combat, Prepared};
use crate::model::{Keywords, Minion, Tribes, Warband};
use crate::rng::Rng;

fn band(minions: Vec<Minion>) -> Warband {
    Warband {
        minions,
        tavern_tier: Some(1),
        values: Default::default(),
    }
}

/// A keyword-only card, so the boards below are fully modelled.
fn plain(attack: i32, health: i32) -> Minion {
    Minion::new("BGS_119", attack, health).tier(1)
}

fn with<R>(player: Vec<Minion>, opponent: Vec<Minion>, f: impl FnOnce(&mut Combat) -> R) -> R {
    let p = Prepared::new(&band(player), &band(opponent), false);
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

fn ids(c: &Combat, side: usize) -> Vec<String> {
    c.sides[side]
        .units
        .iter()
        .map(|u| c.p.cards[u.card as usize].id.clone())
        .collect()
}

fn uid_at(c: &Combat, side: usize, ix: usize) -> u32 {
    c.sides[side].units[ix].uid
}

/// Kills the minions of side 0 at `ixs` at once (a hit from the first
/// enemy) and resolves the deaths.
fn kill(c: &mut Combat, ixs: &[usize]) {
    let by = uid_at(c, 1, 0);
    for &ix in ixs {
        let (uid, health) = (uid_at(c, 0, ix), c.sides[0].units[ix].health);
        c.damage(0, uid, health, Keywords::NONE, by);
    }
    c.process_deaths(1);
}

fn target() -> Vec<Minion> {
    vec![plain(0, 99)]
}

// Timewarped Jazzer: Deathrattle: your Blood Gems give an extra +2 Health
// this game (golden +4).

#[test]
fn timewarped_jazzer_upgrades_the_blood_gems_played_after_it() {
    for (jazzer, extra) in [("BG34_Giant_306", 2), ("BG34_Giant_306_G", 4)] {
        let jazzer = Minion::new(jazzer, 5, 3).tier(3);
        // Tusked Camper: Rally, a Blood Gem on itself.
        let camper = Minion::new("BG33_886", 1, 10).tier(1);
        with(vec![jazzer, camper], target(), |c| {
            kill(c, &[0]);
            assert_eq!(c.sides[0].values.blood_gem, (0, extra));
            let uid = uid_at(c, 0, 0);
            c.swing(0, uid);
            assert_eq!(stats(c, 0), vec![(2, 10 + 1 + extra)]);
        });
    }
}

// Dark Paradox (a tier 5 form): its Divine Shield takes 3 hits to break
// (golden 4).

#[test]
fn dark_paradox_shield_takes_three_hits_and_golden_four() {
    for (id, hits) in [("BG36_360t4", 3), ("BG36_360_Gt4", 4)] {
        let paradox = Minion::new(id, 8, 4)
            .with(Keywords::DIVINE_SHIELD)
            .tribe(Tribes::ALL)
            .tier(5);
        with(vec![paradox], target(), |c| {
            let (uid, by) = (uid_at(c, 0, 0), uid_at(c, 1, 0));
            for _ in 1..hits {
                c.damage(0, uid, 5, Keywords::NONE, by);
                assert!(c.sides[0].units[0].kw.contains(Keywords::DIVINE_SHIELD));
            }
            c.damage(0, uid, 5, Keywords::NONE, by);
            assert!(!c.sides[0].units[0].kw.contains(Keywords::DIVINE_SHIELD));
            assert_eq!(stats(c, 0), vec![(8, 4)]);
        });
    }
}

// Rot Hide Gnoll: has +1 Attack for each friendly minion that died this
// combat (golden +2).

fn gnoll(id: &str, attack: i32) -> Minion {
    Minion::new(id, attack, 4).tribe(Tribes::UNDEAD).tier(1)
}

#[test]
fn rot_hide_gnoll_gains_attack_for_each_friendly_death() {
    for (id, base, per) in [("BG25_013", 1, 1), ("BG25_013_G", 2, 2)] {
        with(
            vec![gnoll(id, base), plain(1, 1), plain(1, 1), plain(1, 5)],
            target(),
            |c| {
                kill(c, &[1, 2]);
                assert_eq!(stats(c, 0), vec![(base + 2 * per, 4), (1, 5)]);
                kill(c, &[1]);
                assert_eq!(stats(c, 0), vec![(base + 3 * per, 4)]);
            },
        );
    }
}

#[test]
fn a_gnoll_that_comes_back_counts_the_deaths_so_far() {
    // The Gnoll has Reborn: it dies with one friend (two deaths), and its
    // fresh copy has the card's 1 Attack and 1 Health plus +1 for each.
    let mut g = gnoll("BG25_013", 3).with(Keywords::REBORN);
    g.base = Some(crate::model::BaseCard {
        attack: 1,
        health: 4,
        keywords: Keywords::NONE,
    });
    with(vec![g, plain(1, 1)], target(), |c| {
        kill(c, &[0, 1]);
        assert_eq!(stats(c, 0), vec![(3, 1)]);
    });
}

// Sewer Lord: Deathrattle: summon two Sewer Rats that summon 2/3 Turtles
// with Taunt (golden: Golden Sewer Rats, 4/6 Turtles).

#[test]
fn sewer_lord_summons_two_rats_that_summon_turtles() {
    for (lord, rat, rat_stats, turtle, turtle_stats) in [
        ("BG35_604", "BG19_010", (3, 2), "BG19_010t", (2, 3)),
        ("BG35_604_G", "BG19_010_G", (6, 4), "BG19_010_Gt", (4, 6)),
    ] {
        let lord = Minion::new(lord, 4, 6).tribe(Tribes::BEAST).tier(5);
        with(vec![lord], target(), |c| {
            kill(c, &[0]);
            assert_eq!(ids(c, 0), vec![rat, rat]);
            assert_eq!(stats(c, 0), vec![rat_stats, rat_stats]);
            kill(c, &[0]);
            assert_eq!(ids(c, 0), vec![turtle, rat]);
            assert_eq!(stats(c, 0)[0], turtle_stats);
            assert!(c.sides[0].units[0].kw.contains(Keywords::TAUNT));
        });
    }
}

// Stalwart Kodo: after you summon a minion in combat, give it this minion's
// maximum stats, 3 times per combat (golden: double).

fn kodo(id: &str, attack: i32, health: i32) -> Minion {
    Minion::new(id, attack, health).tribe(Tribes::BEAST).tier(7)
}

fn bonehead(id: &str) -> Minion {
    Minion::new(id, 1, 1).tribe(Tribes::UNDEAD).tier(1)
}

#[test]
fn stalwart_kodo_gives_summoned_minions_its_maximum_stats() {
    // Bonehead's Deathrattle summons two 1/1 Skeletons: each gets the Kodo's
    // Attack and its full Health (not what the Kodo has left).
    with(
        vec![kodo("BG34_322", 16, 32), bonehead("BG28_300")],
        target(),
        |c| {
            let (kodo, by) = (uid_at(c, 0, 0), uid_at(c, 1, 0));
            c.damage(0, kodo, 20, Keywords::NONE, by);
            kill(c, &[1]);
            assert_eq!(stats(c, 0), vec![(16, 12), (17, 33), (17, 33)]);
        },
    );
}

#[test]
fn golden_kodo_gives_double_and_only_three_times_per_combat() {
    // Golden Bonehead summons four Skeletons: three get double stats.
    with(
        vec![kodo("BG34_322_G", 32, 64), bonehead("BG28_300_G")],
        target(),
        |c| {
            kill(c, &[1]);
            assert_eq!(
                stats(c, 0),
                vec![(32, 64), (65, 129), (65, 129), (65, 129), (1, 1)]
            );
        },
    );
}

// Showy Cyclist: Deathrattle: give all your Naga +2/+1 (golden +4/+2).

#[test]
fn showy_cyclist_buffs_the_nagas_when_it_dies() {
    for (id, (attack, health)) in [("BG31_925", (2, 1)), ("BG31_925_G", (4, 2))] {
        let cyclist = Minion::new(id, 4, 2).tribe(Tribes::NAGA).tier(5);
        let naga = plain(3, 3).tribe(Tribes::NAGA);
        // Minions share a card's facts by its id, so the other tribe needs
        // another card.
        let other = Minion::new("BGS_034", 3, 3).tier(1);
        with(vec![cyclist, naga, other], target(), |c| {
            kill(c, &[0]);
            assert_eq!(stats(c, 0), vec![(3 + attack, 3 + health), (3, 3)]);
        });
    }
}

// Ghastcoiler: Deathrattle: summon 2 random Deathrattle minions (golden 4).

#[test]
fn ghastcoiler_summons_random_deathrattle_minions() {
    for (id, count) in [("BGS_008", 2), ("TB_BaconUps_057", 4)] {
        let coiler = Minion::new(id, 7, 7).tribe(Tribes::BEAST).tier(5);
        with(vec![coiler], target(), |c| {
            kill(c, &[0]);
            let summoned = ids(c, 0);
            assert_eq!(summoned.len(), count);
            for card in &summoned {
                assert!(crate::cards::DEATHRATTLE_POOL.contains(&card.as_str()));
            }
        });
    }
}

#[test]
fn ghastcoiler_summons_only_what_fits_on_the_board() {
    let coiler = Minion::new("TB_BaconUps_057", 7, 7)
        .tribe(Tribes::BEAST)
        .tier(5);
    let mut board = vec![coiler];
    board.extend((0..5).map(|_| plain(1, 9)));
    with(board, target(), |c| {
        kill(c, &[0]);
        assert_eq!(c.sides[0].units.len(), 7);
    });
}
