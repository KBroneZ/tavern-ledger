//! Round 5 cards on small boards whose outcome is certain.

use super::super::{Combat, Prepared};
use crate::cards::{self, Fidelity};
use crate::model::{Minion, Tribes, Warband};
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

fn with_soc<R>(
    player: Vec<Minion>,
    opponent: Vec<Minion>,
    start_of_combat: bool,
    f: impl FnOnce(&mut Combat) -> R,
) -> R {
    let p = Prepared::new(&band(player), &band(opponent), start_of_combat);
    let mut rng = Rng::new(5);
    let mut c = Combat::new(&p, &mut rng);
    f(&mut c)
}

fn with<R>(player: Vec<Minion>, opponent: Vec<Minion>, f: impl FnOnce(&mut Combat) -> R) -> R {
    with_soc(player, opponent, false, f)
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

/// Bonehead: Deathrattle: summon two 1/1 Skeletons.
fn bonehead() -> Minion {
    Minion::new("BG28_300", 1, 1).tribe(Tribes::UNDEAD).tier(1)
}

fn foul_egg() -> Minion {
    Minion::new("BG26_RLK_833", 0, 2)
        .tribe(Tribes::UNDEAD)
        .tier(1)
}

#[test]
fn foul_egg_dies_into_a_3_3_undead_chicken() {
    with(vec![foul_egg()], vec![plain(5, 50)], |c| {
        let egg = uid_at(c, 0, 0);
        c.swing(0, egg);
        assert_eq!(stats(c, 0), [(3, 3)]);
        assert_eq!(ids(c, 0), ["BG_RLK_833t"]);
    });
}

#[test]
fn golden_fish_of_nzoth_gains_each_deathrattle_twice() {
    for id in ["TB_BaconUps_307", "TB_BaconShop_HP_105t_SKIN_A_G"] {
        let fish = Minion::new(id, 4, 4).tribe(Tribes::BEAST).tier(1);
        with(vec![fish, bonehead()], vec![plain(5, 50)], |c| {
            let b = uid_at(c, 0, 1);
            c.swing(0, b);
            assert_eq!(stats(c, 0).len(), 3, "{id}: the Fish and two Skeletons");
            let f = uid_at(c, 0, 0);
            c.swing(0, f);
            // Bonehead's Deathrattle twice: four Skeletons, and the two before.
            assert_eq!(stats(c, 0), [(1, 1); 6], "{id}");
        });
    }
}

#[test]
fn mishmash_gains_what_the_amalgam_gains() {
    let amalgam = Minion::new("TB_BaconShop_HP_033t_SKIN_B", 2, 2).tier(1);
    let mishmash = Minion::new("TB_BaconShop_HERO_33_Buddy", 4, 4).tier(3);
    let golden = Minion::new("TB_BaconShop_HERO_33_Buddy_G", 8, 8).tier(3);
    with(vec![amalgam, mishmash, golden], vec![plain(1, 1)], |c| {
        let a = uid_at(c, 0, 0);
        c.buff(0, a, 1, 2);
        assert_eq!(stats(c, 0), [(3, 4), (5, 6), (10, 12)]);
        // A buff on another minion is not the Amalgam's.
        let m = uid_at(c, 0, 1);
        c.buff(0, m, 1, 1);
        assert_eq!(stats(c, 0), [(3, 4), (6, 7), (10, 12)]);
    });
}

#[test]
fn an_enemy_amalgam_does_not_feed_mishmash() {
    let mishmash = Minion::new("TB_BaconShop_HERO_33_Buddy", 4, 4).tier(3);
    let amalgam = Minion::new("TB_BaconShop_HP_033t", 2, 2).tier(1);
    with(vec![mishmash], vec![amalgam], |c| {
        let a = uid_at(c, 1, 0);
        c.buff(1, a, 5, 5);
        assert_eq!(stats(c, 0), [(4, 4)]);
    });
}

fn whirl(id: &str) -> Minion {
    Minion::new(id, 7, 5).tribe(Tribes::MECH).tier(5)
}

#[test]
fn whirl_o_tron_copies_the_two_left_most_deathrattles_at_start_of_combat() {
    let player = vec![bonehead(), foul_egg(), bonehead(), whirl("BG34_Giant_599")];
    with_soc(player, vec![plain(50, 50)], true, |c| {
        c.start_of_combat(0, [true, false]);
        let w = uid_at(c, 0, 3);
        c.forced_target = Some(uid_at(c, 1, 0));
        c.swing(0, w);
        // Bonehead's and the Egg's Deathrattles, not the second Bonehead's.
        assert_eq!(
            ids(c, 0)[3..],
            ["BG_ICC_026t", "BG_ICC_026t", "BG_RLK_833t"],
            "{:?}",
            ids(c, 0)
        );
    });
}

#[test]
fn golden_whirl_o_tron_copies_twice_and_skips_other_whirl_o_trons() {
    let player = vec![
        whirl("BG34_Giant_599"),
        bonehead(),
        whirl("BG34_Giant_599_G"),
    ];
    with_soc(player, vec![plain(50, 50)], true, |c| {
        c.start_of_combat(0, [true, false]);
        // The first copies Bonehead once; the golden one, Bonehead twice
        // (never the first Whirl-O-Tron, whose Deathrattle is a copy).
        assert_eq!(c.sides[0].units[0].gained.len(), 1);
        assert_eq!(c.sides[0].units[2].gained.len(), 2);
    });
}

#[test]
fn whirl_o_tron_on_a_board_after_start_of_combat_copies_nothing() {
    with(
        vec![bonehead(), whirl("BG34_Giant_599")],
        vec![plain(50, 50)],
        |c| {
            let w = uid_at(c, 0, 1);
            c.swing(0, w);
            assert_eq!(stats(c, 0), [(1, 1)]);
        },
    );
}

#[test]
fn hand_summoners_play_as_approximate_with_the_summon_left_out() {
    for id in cards::HAND_SUMMONERS {
        let rules = cards::rules(id).unwrap_or_else(|| panic!("{id} is not modelled"));
        assert!(
            matches!(rules.fidelity, Fidelity::Approximate(_)),
            "{id} is approximate"
        );
        assert!(
            rules.triggers.is_empty(),
            "{id}: the hand summon is left out"
        );
    }
}

#[test]
fn salvager_and_festergut_stay_not_modelled_and_say_why() {
    // Ghastcoiler was left out in round 5 and is played since round 8.
    for id in [
        "BG31_999",
        "BG31_999_G",
        "BG34_Giant_590",
        "BG34_Giant_590_G",
    ] {
        assert_eq!(cards::rules(id), None, "{id}");
        assert!(!cards::refusal(id).contains("not reviewed"), "{id}");
    }
}

#[test]
fn a_dead_amalgam_and_an_empty_buff_feed_nothing() {
    let amalgam = Minion::new("TB_BaconShop_HP_033t", 2, 2).tier(1);
    let mishmash = Minion::new("TB_BaconShop_HERO_33_Buddy", 4, 4).tier(3);
    with(vec![amalgam, mishmash], vec![plain(1, 1)], |c| {
        let a = uid_at(c, 0, 0);
        c.buff(0, a, 0, 0);
        c.sides[0].units[0].health = 0;
        c.buff(0, a, 3, 0);
        assert_eq!(stats(c, 0)[1], (4, 4));
    });
}

#[test]
fn golden_fish_passes_what_a_dead_fish_gained_twice() {
    let fish = Minion::new("TB_BaconShop_HP_105t", 2, 2).tier(1);
    let golden = Minion::new("TB_BaconUps_307", 4, 40).tier(1);
    with(vec![golden, fish, bonehead()], vec![plain(5, 50)], |c| {
        // Bonehead dies: the Fish gains it once, the golden Fish twice.
        let b = uid_at(c, 0, 1 + 1);
        c.swing(0, b);
        assert_eq!(c.sides[0].units[0].gained.len(), 2);
        assert_eq!(c.sides[0].units[1].gained.len(), 1);
        // The Fish dies: the golden Fish gains what it had gained, twice.
        let f = uid_at(c, 0, 1);
        c.swing(0, f);
        assert_eq!(c.sides[0].units[0].gained.len(), 4);
    });
}

#[test]
fn whirl_o_tron_skips_minions_without_a_deathrattle_and_copies_gained_ones() {
    let fish = Minion::new("TB_BaconShop_HP_105t", 2, 2).tier(1);
    let player = vec![plain(1, 1), fish, whirl("BG34_Giant_599")];
    // A Bonehead on the enemy board puts its card in the combat's card list.
    let opponent = vec![bonehead(), plain(50, 50)];
    with_soc(player, opponent, true, |c| {
        let bonehead = c.p.cards.iter().position(|k| k.id == "BG28_300").unwrap() as u16;
        // The Fish gained Bonehead's Deathrattle earlier in this combat.
        c.sides[0].units[1].gained.push(bonehead);
        c.start_of_combat(0, [true, false]);
        // The plain minion is skipped; the Fish's gained Deathrattle is copied.
        assert_eq!(c.sides[0].units[2].gained, [bonehead]);
    });
}
