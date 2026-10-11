//! Round 2 cards on small boards whose outcome is certain.

use super::super::{Combat, Prepared};
use crate::model::{BaseCard, Keywords, Minion, Tribes, Warband};
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

/// A minion that comes back once with its own stats.
fn reborn(attack: i32, health: i32, tribe: Tribes) -> Minion {
    let mut m = plain(attack, health).tribe(tribe).with(Keywords::REBORN);
    m.base = Some(BaseCard {
        attack,
        health,
        keywords: Keywords::REBORN,
    });
    m
}

/// Runs `f` on a combat between the two boards (side 0 is the player).
fn with<R>(player: Vec<Minion>, opponent: Vec<Minion>, f: impl FnOnce(&mut Combat) -> R) -> R {
    let p = Prepared::new(&band(player), &band(opponent), false);
    let mut rng = Rng::new(5);
    let mut c = Combat::new(&p, &mut rng);
    f(&mut c)
}

fn uid(c: &Combat, side: usize, ix: usize) -> u32 {
    c.sides[side].units[ix].uid
}

fn stats(c: &Combat, side: usize) -> Vec<(i32, i32)> {
    c.sides[side]
        .units
        .iter()
        .map(|u| (u.attack, u.health))
        .collect()
}

#[test]
fn cage_gnawer_buffs_beasts_when_it_attacks_itself() {
    let gnawer = Minion::new("BG36_211", 2, 7).tribe(Tribes::BEAST).tier(4);
    with(vec![gnawer, plain(1, 1)], vec![plain(0, 10)], |c| {
        let g = uid(c, 0, 0);
        c.swing(0, g);
        assert_eq!(stats(c, 0), [(4, 8), (1, 1)], "only Beasts");
        assert_eq!(stats(c, 1), [(0, 6)], "hit with 4 Attack");
    });
}

#[test]
fn barrier_banshee_gains_after_a_reborn() {
    let banshee = Minion::new("BG36_514", 8, 8).tribe(Tribes::UNDEAD).tier(5);
    with(
        vec![reborn(1, 1, Tribes::UNDEAD), banshee],
        vec![plain(5, 5), plain(5, 5)],
        |c| {
            let target = uid(c, 0, 0);
            c.forced_target = Some(target);
            let foe = uid(c, 1, 0);
            c.swing(1, foe);
            let b = &c.sides[0].units[1];
            assert_eq!((b.attack, b.health), (16, 16));
            assert!(b.kw.contains(Keywords::DIVINE_SHIELD));
        },
    );
}

#[test]
fn snazzy_phantom_gives_the_reborn_attack_to_the_right_most_undead() {
    let phantom = Minion::new("BG36_515", 6, 8).tribe(Tribes::UNDEAD).tier(6);
    // Card facts are shared by card id: each test minion has its own id.
    let right = Minion::new("TEST_UNDEAD", 1, 1)
        .tribe(Tribes::UNDEAD)
        .tier(1);
    let other = Minion::new("TEST_OTHER", 2, 2).tier(1);
    let board = vec![reborn(3, 1, Tribes::UNDEAD), phantom, right, other];
    with(board, vec![plain(5, 5), plain(5, 5)], |c| {
        c.forced_target = Some(uid(c, 0, 0));
        let foe = uid(c, 1, 0);
        c.swing(1, foe);
        // The 3/1 came back; the right-most Undead got +3/+3.
        assert_eq!(stats(c, 0), [(3, 1), (6, 8), (4, 4), (2, 2)]);
    });
}

#[test]
fn eternal_tycoon_summons_a_knight_that_attacks_at_once() {
    let tycoon = Minion::new("BG34_403", 4, 8).tribe(Tribes::UNDEAD).tier(5);
    with(
        vec![tycoon, plain(1, 1)],
        vec![plain(1, 20), plain(1, 20)],
        |c| {
            c.sides[0].units[0].avenge = 4;
            c.forced_target = Some(uid(c, 0, 1));
            let foe = uid(c, 1, 0);
            c.swing(1, foe);
            // The 1/1 died: the fifth death. The Knight (4/2) hit a 1/20 back.
            assert_eq!(c.sides[0].units.len(), 2, "Tycoon and Knight");
            assert_eq!(c.p.cards[c.sides[0].units[1].card as usize].id, "BG25_008");
            // 1 back from the 1/1, 4 from the Knight.
            let foes: i32 = c.sides[1].units.iter().map(|u| u.health).sum();
            assert_eq!(foes, 35);
        },
    );
}

#[test]
fn the_last_one_standing_gives_one_minion_of_each_type() {
    let last = Minion::new("BG34_320", 15, 15).tribe(Tribes::ALL).tier(7);
    with(vec![last], vec![plain(0, 500)], |c| {
        let l = uid(c, 0, 0);
        c.swing(0, l);
        // Alone, it is every type: eleven times +15/+15.
        assert_eq!(stats(c, 0), [(180, 180)]);
    });
}

#[test]
fn devout_hellcaller_grows_when_another_demon_deals_damage() {
    let caller = Minion::new("BG33_155", 4, 4).tribe(Tribes::DEMON).tier(3);
    let demon = plain(2, 9).tribe(Tribes::DEMON);
    with(
        vec![demon.clone(), caller.clone()],
        vec![plain(0, 9)],
        |c| {
            let d = uid(c, 0, 0);
            c.swing(0, d);
            assert_eq!(stats(c, 0)[1], (6, 6));
        },
    );
    let shielded = plain(0, 9).with(Keywords::DIVINE_SHIELD);
    with(vec![demon, caller], vec![shielded], |c| {
        let d = uid(c, 0, 0);
        c.swing(0, d);
        assert_eq!(stats(c, 0)[1], (4, 4), "a Divine Shield took the hit");
    });
}

#[test]
fn deathstrider_triggers_the_left_most_deathrattle_after_a_rally_attack() {
    let rally = Minion::new("BG36_200", 1, 5).tribe(Tribes::BEAST).tier(1);
    let rattle = Minion::new("BG29_611", 1, 5).tribe(Tribes::MECH).tier(1);
    let strider = Minion::new("BG36_208", 10, 11).tribe(Tribes::BEAST).tier(6);
    with(vec![rally, rattle, strider], vec![plain(0, 50)], |c| {
        let r = uid(c, 0, 0);
        c.swing(0, r);
        let ids: Vec<&str> = c.sides[0]
            .units
            .iter()
            .map(|u| c.p.cards[u.card as usize].id.as_str())
            .collect();
        // Rally summoned a 1/1 Beast; then the Mech's Deathrattle summoned
        // a 1/1 Mech next to it.
        assert_eq!(
            ids,
            [
                "BG36_200",
                "BG36_200t",
                "BG29_611",
                "BG_BOT_312t",
                "BG36_208"
            ]
        );
    });
}

#[test]
fn crimson_vindicator_casts_mighty_dragonbreath() {
    let vindicator = Minion::new("BG36_241", 8, 9)
        .tribe(Tribes::DRAGON)
        .with(Keywords::DIVINE_SHIELD)
        .tier(6);
    with(vec![vindicator, plain(1, 1)], vec![plain(0, 99)], |c| {
        let v = uid(c, 0, 0);
        c.swing(0, v);
        // Dragon with Divine Shield: three times +3/+2; the plain one once.
        assert_eq!(stats(c, 0), [(17, 15), (4, 3)]);
    });
}
