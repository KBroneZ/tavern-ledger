//! Round 4 cards on small boards whose outcome is certain.

use super::super::{Combat, Prepared};
use crate::model::{Keywords, Minion, Tribes, Warband};
use crate::rng::Rng;

fn band(minions: Vec<Minion>) -> Warband {
    Warband {
        minions,
        tavern_tier: Some(1),
    }
}

fn plain(attack: i32, health: i32) -> Minion {
    Minion::new("TEST_PLAIN", attack, health).tier(1)
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

fn ids(c: &Combat, side: usize) -> Vec<String> {
    c.sides[side]
        .units
        .iter()
        .map(|u| c.p.cards[u.card as usize].id.clone())
        .collect()
}

fn first(c: &Combat, side: usize) -> u32 {
    c.sides[side].units[0].uid
}

/// Bonehead: Deathrattle: summon two 1/1 Skeletons.
fn bonehead() -> Minion {
    Minion::new("BG28_300", 1, 1).tribe(Tribes::UNDEAD).tier(1)
}

#[test]
fn dark_paradox_dies_into_a_golem_with_its_full_stats() {
    let paradox = Minion::new("BG36_360t9", 10, 4).tier(6);
    with(vec![paradox], vec![plain(5, 50)], |c| {
        let p = first(c, 0);
        c.swing(0, p);
        // Its full Health (4), not what was left.
        assert_eq!(stats(c, 0), [(10, 4)]);
        assert_eq!(ids(c, 0), ["BG30_MagicItem_442t"]);
    });
}

#[test]
fn golden_dark_paradox_doubles_stats_gained_in_combat() {
    let paradox = Minion::new("BG36_360_Gt9", 20, 4).tier(6);
    // A buff in combat raises the full Health too.
    with(vec![paradox], vec![plain(9, 50)], |c| {
        let p = first(c, 0);
        c.buff(0, p, 1, 2);
        c.swing(0, p);
        assert_eq!(stats(c, 0), [(42, 12)]);
    });
}

#[test]
fn jailbird_juggernauts_golem_hits_the_target_first() {
    let jailbird = Minion::new("BG36_333", 6, 15).tier(7);
    with(vec![jailbird], vec![plain(1, 20)], |c| {
        let j = first(c, 0);
        c.swing(0, j);
        // Golem 6/15 hits first, then the Juggernaut: 20 - 12.
        assert_eq!(stats(c, 1), [(1, 8)]);
        assert_eq!(stats(c, 0), [(6, 14), (6, 14)]);
        assert_eq!(ids(c, 0), ["BG36_333", "BG30_MagicItem_442t"]);
    });
}

#[test]
fn fish_of_nzoth_gains_a_dying_friends_deathrattle() {
    let fish = Minion::new("TB_BaconShop_HP_105t", 2, 2)
        .tribe(Tribes::BEAST)
        .tier(1);
    with(vec![fish, bonehead()], vec![plain(5, 50)], |c| {
        let b = c.sides[0].units[1].uid;
        c.swing(0, b);
        assert_eq!(stats(c, 0), [(2, 2), (1, 1), (1, 1)]);
        let f = first(c, 0);
        c.swing(0, f);
        // The Fish dies with Bonehead's Deathrattle: two Skeletons.
        assert_eq!(stats(c, 0), [(1, 1), (1, 1), (1, 1), (1, 1)]);
    });
}

#[test]
fn fish_of_nzoth_without_a_gain_summons_nothing() {
    let fish = Minion::new("TB_BaconShop_HP_105t", 2, 2).tier(1);
    with(vec![fish], vec![bonehead(), plain(5, 50)], |c| {
        // The enemy's Deathrattle is not the Fish's to gain.
        c.sides[1].units[0].health = 0;
        c.process_deaths(1);
        let f = first(c, 0);
        c.forced_target = Some(c.sides[1].units.last().unwrap().uid);
        c.swing(0, f);
        assert!(stats(c, 0).is_empty());
    });
}

#[test]
fn ravaging_scorpid_buffs_beetles_after_a_friend_attacks() {
    let scorpid = Minion::new("BG36_209", 6, 7).tribe(Tribes::BEAST).tier(6);
    let beetle = Minion::new("BG28_603t", 2, 2).tribe(Tribes::BEAST).tier(1);
    with(
        vec![plain(1, 10), scorpid, beetle],
        vec![plain(0, 50)],
        |c| {
            let a = first(c, 0);
            c.swing(0, a);
            assert_eq!(stats(c, 0), [(1, 10), (6, 7), (6, 6)]);
        },
    );
}

#[test]
fn golden_ravaging_scorpid_dies_into_two_beetles() {
    let scorpid = Minion::new("BG36_209_G", 12, 14).tier(6);
    with(vec![scorpid], vec![plain(20, 50)], |c| {
        let s = first(c, 0);
        c.swing(0, s);
        assert_eq!(ids(c, 0), ["BG28_603t", "BG28_603t"]);
        assert_eq!(stats(c, 0), [(2, 2), (2, 2)]);
    });
}

#[test]
fn warghoul_triggers_its_neighbours_deathrattle() {
    let warghoul = taunt(Minion::new("BG34_Giant_331", 9, 3).tier(5));
    with(vec![warghoul, bonehead()], vec![plain(5, 50)], |c| {
        let w = first(c, 0);
        c.swing(0, w);
        // Bonehead lives; its Deathrattle summons two Skeletons right of it.
        assert_eq!(stats(c, 0), [(1, 1), (1, 1), (1, 1)]);
        assert_eq!(ids(c, 0), ["BG28_300", "BG_ICC_026t", "BG_ICC_026t"]);
    });
}

#[test]
fn golden_warghoul_triggers_both_neighbours_but_not_another_warghoul() {
    let warghoul = || taunt(Minion::new("BG34_Giant_331_G", 18, 6).tier(5));
    with(
        vec![bonehead(), warghoul(), warghoul()],
        vec![plain(10, 50)],
        |c| {
            let w = c.sides[0].units[1].uid;
            c.swing(0, w);
            // Only Bonehead's Deathrattle: the other Warghoul is left out.
            assert_eq!(
                ids(c, 0),
                ["BG28_300", "BG_ICC_026t", "BG_ICC_026t", "BG34_Giant_331_G"]
            );
        },
    );
}

#[test]
fn golden_wildfire_sends_excess_damage_to_both_neighbours() {
    let wildfire = Minion::new("TB_BaconUps_166", 12, 6).tier(3);
    let middle = taunt(plain(0, 2));
    with(
        vec![wildfire],
        vec![plain(0, 20), middle, plain(0, 20)],
        |c| {
            let w = first(c, 0);
            c.swing(0, w);
            assert_eq!(stats(c, 1), [(0, 10), (0, 10)]);
        },
    );
}

#[test]
fn colossus_damages_the_targets_neighbours() {
    let colossus = Minion::new("BG31_HERO_802pt", 6, 12).tier(5);
    with(
        vec![colossus],
        vec![plain(0, 5), taunt(plain(0, 20)), plain(0, 5)],
        |c| {
            let x = first(c, 0);
            c.swing(0, x);
            assert_eq!(stats(c, 1), [(0, 3), (0, 14), (0, 3)]);
        },
    );
}

#[test]
fn spirit_of_air_gives_a_friend_three_keywords() {
    let spirit = Minion::new("TB_BaconShop_HERO_76_Buddy", 5, 2).tier(1);
    with(vec![spirit, plain(1, 10)], vec![plain(5, 50)], |c| {
        let s = first(c, 0);
        c.swing(0, s);
        let kw = c.sides[0].units[0].kw;
        for k in [Keywords::WINDFURY, Keywords::DIVINE_SHIELD, Keywords::TAUNT] {
            assert!(kw.contains(k));
        }
    });
}

#[test]
fn carrier_summons_an_interceptor_after_four_friends_die() {
    let carrier = Minion::new("BG31_HERO_802pt1", 4, 12).tier(5);
    let small = || plain(1, 1);
    with(
        vec![small(), small(), small(), small(), carrier],
        vec![plain(1, 50)],
        |c| {
            for u in c.sides[0].units.iter_mut().take(4) {
                u.health = 0;
            }
            c.process_deaths(0);
            assert_eq!(stats(c, 0), [(4, 12), (7, 7)]);
            assert_eq!(ids(c, 0), ["BG31_HERO_802pt1", "BG31_HERO_802pt1t"]);
        },
    );
}

#[test]
fn golden_jailbird_juggernauts_golem_has_double_stats() {
    let jailbird = Minion::new("BG36_333_G", 12, 30).tier(7);
    with(vec![jailbird], vec![plain(1, 100)], |c| {
        let j = first(c, 0);
        c.swing(0, j);
        // Golem 24/60, then the Juggernaut: 100 - 36.
        assert_eq!(stats(c, 1), [(1, 64)]);
        assert_eq!(stats(c, 0), [(12, 29), (24, 59)]);
    });
}

#[test]
fn jailbird_does_not_attack_when_its_golem_killed_the_target() {
    // Not checked against the game (the card is approximate): the
    // Juggernaut's own hit is lost and its after-attack triggers do not run.
    let jailbird = Minion::new("BG36_333", 6, 15).tier(7);
    let scorpid = Minion::new("BG36_209", 6, 7).tribe(Tribes::BEAST).tier(6);
    let beetle = Minion::new("BG28_603t", 2, 2).tribe(Tribes::BEAST).tier(1);
    with(vec![jailbird, scorpid, beetle], vec![plain(0, 5)], |c| {
        let j = first(c, 0);
        c.swing(0, j);
        assert!(c.sides[1].units.is_empty());
        // One friendly attack (the Golem's): the Beetle gets +4/+4 once.
        assert_eq!(stats(c, 0), [(6, 15), (6, 15), (6, 7), (6, 6)]);
    });
}

#[test]
fn golem_is_lost_on_a_full_board() {
    let jailbird = Minion::new("BG36_333", 6, 15).tier(7);
    let mut board = vec![jailbird];
    board.extend((0..6).map(|_| plain(1, 10)));
    with(board, vec![plain(0, 50)], |c| {
        let j = first(c, 0);
        c.swing(0, j);
        assert_eq!(c.sides[0].units.len(), 7);
        assert_eq!(stats(c, 1), [(0, 44)], "only the Juggernaut hits");
    });
}

#[test]
fn warghoul_last_on_the_board_triggers_its_left_neighbour() {
    let warghoul = taunt(Minion::new("BG34_Giant_331", 9, 3).tier(5));
    with(vec![bonehead(), warghoul], vec![plain(5, 50)], |c| {
        let w = c.sides[0].units[1].uid;
        c.swing(0, w);
        assert_eq!(ids(c, 0), ["BG28_300", "BG_ICC_026t", "BG_ICC_026t"]);
    });
}

#[test]
fn a_fish_passes_what_it_gained_to_another_fish() {
    let fish = || {
        Minion::new("TB_BaconShop_HP_105t", 2, 2)
            .tribe(Tribes::BEAST)
            .tier(1)
    };
    with(vec![fish(), bonehead(), fish()], vec![plain(5, 50)], |c| {
        let b = c.sides[0].units[1].uid;
        c.swing(0, b);
        let f = first(c, 0);
        c.swing(0, f);
        // The second Fish has Bonehead's Deathrattle once, not the Fish card.
        let last = c.sides[0].units.last().unwrap();
        assert_eq!(c.p.cards[last.card as usize].id, "TB_BaconShop_HP_105t");
        assert_eq!(last.gained.len(), 2, "from Bonehead, then from the Fish");
        let bone = c.p.cards.iter().position(|x| x.id == "BG28_300").unwrap();
        assert!(last.gained.iter().all(|g| usize::from(*g) == bone));
    });
}

#[test]
fn windfury_runs_after_friendly_attack_once_per_swing() {
    let attacker = Minion {
        keywords: Keywords::WINDFURY,
        ..plain(1, 10)
    };
    let scorpid = Minion::new("BG36_209", 6, 7).tribe(Tribes::BEAST).tier(6);
    let beetle = Minion::new("BG28_603t", 2, 2).tribe(Tribes::BEAST).tier(1);
    with(vec![attacker, scorpid, beetle], vec![plain(0, 50)], |c| {
        c.attack_turn(0);
        assert_eq!(stats(c, 0)[2], (10, 10));
    });
}
