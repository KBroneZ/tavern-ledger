//! T-628 round 4: the hero powers and trinkets played during combat, and
//! Zerg's morphing minions, on small boards whose outcome is certain.

use super::{Combat, Prepared};
use crate::model::{GameValues, Keywords, Kit, KitCard, Minion, Team, Tribes, Warband};
use crate::rng::Rng;

fn plain(attack: i32, health: i32) -> Minion {
    Minion::new("BGS_119", attack, health).tier(1)
}

fn kitted(minions: Vec<Minion>, powers: &[&str], trinkets: &[&str]) -> Warband {
    Warband {
        kit: Some(Kit {
            powers: powers.iter().map(|id| KitCard::new(*id)).collect(),
            trinkets: trinkets.iter().map(|id| KitCard::new(*id)).collect(),
            round: Some(9),
        }),
        tavern_tier: Some(1),
        ..Warband::new(minions)
    }
}

/// A combat between the two boards (start of combat `soc`), not opened yet.
fn with(player: Warband, opponent: Vec<Minion>, soc: bool, f: impl FnOnce(&mut Combat)) {
    let opponent = kitted(opponent, &[], &[]);
    let p = Prepared::teams(&Team::solo(player), &Team::solo(opponent), soc);
    let mut rng = Rng::new(3);
    let mut c = Combat::new(&p, &mut rng);
    f(&mut c);
}

fn stats(c: &Combat, side: usize) -> Vec<(i32, i32)> {
    c.sides[side]
        .units
        .iter()
        .map(|u| (u.attack, u.health))
        .collect()
}

fn uid(c: &Combat, side: usize, ix: usize) -> u32 {
    c.sides[side].units[ix].uid
}

#[test]
fn glory_of_combat_gives_the_killer_one_attack_attacking_or_attacked() {
    let player = kitted(vec![plain(5, 9), plain(1, 9)], &["BG20_HERO_100p"], &[]);
    with(player, vec![plain(1, 3), plain(2, 1)], false, |c| {
        // The player's 5/9 kills the 1/3: +1 Attack.
        let (a, t) = (uid(c, 0, 0), uid(c, 1, 0));
        c.forced_target = Some(t);
        c.swing(0, a);
        assert_eq!(stats(c, 0)[0], (6, 8));
        // The enemy 2/1 attacks the 1/9 and dies on it: the 1/9 gains.
        let (a, t) = (uid(c, 1, 0), uid(c, 0, 1));
        c.forced_target = Some(t);
        c.swing(1, a);
        assert_eq!(stats(c, 0)[1], (2, 7));
    });
}

#[test]
fn glory_of_combat_gives_nothing_to_a_killer_that_died_too() {
    let player = kitted(vec![plain(3, 3)], &["BG20_HERO_100p"], &[]);
    with(player, vec![plain(3, 3), plain(0, 9)], false, |c| {
        let (a, t) = (uid(c, 0, 0), uid(c, 1, 0));
        c.forced_target = Some(t);
        c.swing(0, a);
        assert!(c.sides[0].units.is_empty());
    });
}

#[test]
fn jar_o_gems_plays_a_blood_gem_on_every_quilboar_every_second_attack() {
    let boar = Minion::new("ZZ_BOAR", 1, 9).tribe(Tribes::QUILBOAR);
    let mut player = kitted(
        vec![boar.clone(), boar, plain(1, 9)],
        &[],
        &["BG30_MagicItem_546"],
    );
    player.values = GameValues {
        blood_gem: (1, 2),
        ..GameValues::default()
    };
    with(player, vec![plain(0, 50)], false, |c| {
        let a = uid(c, 0, 2);
        c.swing(0, a);
        assert_eq!(stats(c, 0)[0], (1, 9));
        c.swing(0, a);
        // +1/+1 and the side's upgrades (+1/+2): on both Quilboar only.
        assert_eq!(stats(c, 0), [(3, 12), (3, 12), (1, 9)]);
        c.swing(0, a);
        c.swing(0, a);
        assert_eq!(stats(c, 0)[0], (5, 15));
    });
}

#[test]
fn deathly_phylactery_makes_the_first_deathrattle_trigger_twice() {
    // Two Sewer Rats: the first to die summons two Turtles, the second one.
    let rat = || Minion::new("BG19_010", 3, 2).tribe(Tribes::BEAST).tier(2);
    let player = kitted(vec![rat(), rat()], &[], &["BG30_MagicItem_700"]);
    with(player, vec![plain(0, 50)], true, |c| {
        c.sides[0].units[0].health = 0;
        c.process_deaths(0);
        assert_eq!(c.sides[0].units.len(), 3, "rat and two Turtles");
        c.sides[0].units[2].health = 0;
        c.process_deaths(0);
        assert_eq!(c.sides[0].units.len(), 3, "two Turtles and one more");
    });
    // On a board taken after start of combat it is not played.
    let player = kitted(vec![rat()], &[], &["BG30_MagicItem_700"]);
    with(player, vec![plain(0, 50)], false, |c| {
        c.sides[0].units[0].health = 0;
        c.process_deaths(0);
        assert_eq!(c.sides[0].units.len(), 1);
    });
}

#[test]
fn sprout_it_out_gives_what_the_side_summons_stats_and_taunt() {
    let rat = Minion::new("BG19_010", 3, 2).tribe(Tribes::BEAST).tier(2);
    let player = kitted(vec![rat], &["TB_BaconShop_HP_107"], &[]);
    with(player, vec![plain(0, 50)], false, |c| {
        c.sides[0].units[0].health = 0;
        c.process_deaths(0);
        assert_eq!(stats(c, 0), [(3, 5)], "a 2/3 Turtle with +1/+2");
        assert!(c.sides[0].units[0].kw.contains(Keywords::TAUNT));
    });
}

#[test]
fn lionfish_portrait_and_faerie_dragon_scale_act_on_their_type_attacking() {
    let beast = Minion::new("ZZ_BEAST", 1, 9).tribe(Tribes::BEAST);
    let player = kitted(vec![beast, plain(1, 9)], &[], &["BG36_MagicItem_201"]);
    with(player, vec![plain(0, 50)], false, |c| {
        let (b, o) = (uid(c, 0, 0), uid(c, 0, 1));
        c.swing(0, b);
        c.swing(0, o);
        assert_eq!(stats(c, 0), [(5, 11), (1, 9)]);
    });
    let dragon = Minion::new("ZZ_DRAGON", 1, 9).tribe(Tribes::DRAGON);
    let player = kitted(vec![dragon], &[], &["BG32_MagicItem_363"]);
    with(player, vec![plain(1, 50)], false, |c| {
        let d = uid(c, 0, 0);
        for _ in 0..3 {
            c.swing(0, d);
        }
        // The shield took each counter-hit: three times per combat.
        assert_eq!(stats(c, 0), [(1, 9)]);
        c.swing(0, d);
        assert_eq!(stats(c, 0), [(1, 8)]);
    });
}

#[test]
fn staff_and_beetle_band_avenge_friendly_deaths() {
    let undead = Minion::new("ZZ_UNDEAD", 1, 9).tribe(Tribes::UNDEAD);
    let mut minions = vec![plain(1, 1); 5];
    minions.push(undead);
    let player = kitted(minions, &[], &["BG30_MagicItem_437"]);
    with(player, vec![plain(0, 50)], false, |c| {
        for _ in 0..5 {
            c.sides[0].units[0].health = 0;
            c.process_deaths(0);
        }
        assert!(c.sides[0].units[0].kw.contains(Keywords::REBORN));
    });
    let player = kitted(vec![plain(1, 1); 7], &[], &["BG32_MagicItem_860t"]);
    with(player, vec![plain(0, 50)], false, |c| {
        for _ in 0..7 {
            c.sides[0].units[0].health = 0;
            c.process_deaths(0);
        }
        assert_eq!(stats(c, 0), [(2, 2), (2, 2)]);
        assert!(c.sides[0]
            .units
            .iter()
            .all(|u| u.kw.contains(Keywords::TAUNT)));
    });
}

#[test]
fn zergs_baneling_and_lurker() {
    let baneling = Minion::new("BG31_HERO_811t5", 4, 1).tier(2);
    let lurker = Minion::new("BG31_HERO_811t7", 2, 2).tier(2);
    let player = kitted(vec![baneling, lurker], &[], &[]);
    with(player, vec![plain(0, 9)], false, |c| {
        c.sides[0].units[0].health = 0;
        c.process_deaths(0);
        assert_eq!(stats(c, 1), [(0, 5)], "its 4 Attack to the enemy");
        assert_eq!(stats(c, 0), [(3, 3)], "Avenge (1): +1/+1");
    });
}

#[test]
fn golden_baneling_hits_twice_and_golden_lurker_gains_double() {
    let baneling = Minion::new("BG31_HERO_811t5_G", 3, 1).tier(2);
    let lurker = Minion::new("BG31_HERO_811t7_G", 4, 4).tier(2);
    let player = kitted(vec![baneling, lurker], &[], &[]);
    with(player, vec![plain(0, 9)], false, |c| {
        c.sides[0].units[0].health = 0;
        c.process_deaths(0);
        assert_eq!(stats(c, 1), [(0, 3)], "3 Attack twice");
        assert_eq!(stats(c, 0), [(6, 6)], "Avenge (1): +2/+2");
    });
}

/// A Duos combat: the player's two boards against one board.
fn duos(first: Warband, second: Warband, opponent: Vec<Minion>, f: impl FnOnce(&mut Combat)) {
    let opponent = kitted(opponent, &[], &[]);
    let p = Prepared::teams(&Team::duo(first, second), &Team::solo(opponent), false);
    let mut rng = Rng::new(3);
    let mut c = Combat::new(&p, &mut rng);
    f(&mut c);
}

#[test]
fn a_duos_board_that_comes_in_counts_again_with_its_own_kit() {
    let boar = || Minion::new("ZZ_BOAR", 1, 9).tribe(Tribes::QUILBOAR);
    // The first board attacks once with Jar o' Gems, then is gone.
    let first = kitted(vec![boar()], &[], &["BG30_MagicItem_546"]);
    let second = kitted(vec![boar(), plain(1, 9)], &[], &["BG30_MagicItem_546"]);
    duos(first, second, vec![plain(0, 50)], |c| {
        let a = uid(c, 0, 0);
        c.swing(0, a);
        c.sides[0].units.clear();
        c.bring_in_teammates();
        // One attack of the second board: its count starts at 0, no gems.
        let a = uid(c, 0, 1);
        c.swing(0, a);
        assert_eq!(stats(c, 0)[0], (1, 9));
        c.swing(0, a);
        assert_eq!(stats(c, 0)[0], (2, 10));
    });
    // Glory of Combat on the first board does nothing for the second.
    let first = kitted(vec![plain(1, 1)], &["BG20_HERO_100p"], &[]);
    let second = kitted(vec![plain(5, 9)], &[], &[]);
    duos(first, second, vec![plain(1, 3), plain(0, 9)], |c| {
        c.sides[0].units.clear();
        c.bring_in_teammates();
        let (a, t) = (uid(c, 0, 0), uid(c, 1, 0));
        c.forced_target = Some(t);
        c.swing(0, a);
        assert_eq!(stats(c, 0), [(5, 8)]);
    });
}
