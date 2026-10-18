//! T-628 round 3: the hero powers that act once their side has space
//! (Frostwolf Fervor, Stormpike Strength, Lock and Load, Rapid Reanimation's
//! copy), the targeted ones (Rapid Reanimation, Glorious Gloop) and the
//! minions that make exact copies (Stitched Salvager, Timewarped Chameleon,
//! Sandy), on small boards whose outcome is certain.

use super::{Combat, Prepared};
use crate::model::{Keywords, Kit, KitCard, Minion, Team, Warband, MAX_BOARD};
use crate::rng::Rng;

fn plain(attack: i32, health: i32) -> Minion {
    Minion::new("BGS_119", attack, health).tier(1)
}

fn power(id: &str, ready: bool) -> KitCard {
    let mut card = KitCard::new(id);
    card.counters[0] = i32::from(ready);
    card
}

fn band(minions: Vec<Minion>, powers: Vec<KitCard>, round: u32) -> Warband {
    Warband {
        kit: Some(Kit {
            powers,
            trinkets: Vec::new(),
            round: Some(round),
        }),
        tavern_tier: Some(1),
        ..Warband::new(minions)
    }
}

/// Plays start of combat (the player first) on the two teams and hands the
/// combat over.
fn after_start(player: Team, opponent: Team, f: impl FnOnce(&mut Combat)) {
    let p = Prepared::teams(&player, &opponent, true);
    let mut rng = Rng::new(5);
    let mut c = Combat::new(&p, &mut rng);
    c.start_of_combat(0, [true, true]);
    f(&mut c);
}

fn solo(player: Warband, opponent: Vec<Minion>, f: impl FnOnce(&mut Combat)) {
    let opponent = band(opponent, Vec::new(), 7);
    after_start(Team::solo(player), Team::solo(opponent), f);
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

/// Kills the minion at board index `ix` of `side` and resolves the deaths.
fn kill(c: &mut Combat, side: usize, ix: usize) {
    c.sides[side].units[ix].health = 0;
    c.process_deaths(0);
}

#[test]
fn frostwolf_fervor_copies_the_highest_attack_minion_to_its_right() {
    let minions = vec![plain(3, 2), plain(5, 1), plain(5, 9), plain(1, 1)];
    let player = band(minions, vec![power("BG22_HERO_002p", true)], 7);
    solo(player, vec![plain(0, 9)], |c| {
        // The left-most of the two with 5 Attack.
        assert_eq!(stats(c, 0), [(3, 2), (5, 1), (5, 1), (5, 9), (1, 1)]);
        let uids: Vec<u32> = c.sides[0].units.iter().map(|u| u.uid).collect();
        assert_ne!(uids[1], uids[2], "a new minion");
    });
}

#[test]
fn stormpike_strength_copies_the_highest_health_minion_with_its_keywords() {
    let shielded = plain(2, 8).with(Keywords::DIVINE_SHIELD);
    let player = band(
        vec![plain(9, 1), shielded],
        vec![power("BG22_HERO_003p", true)],
        9,
    );
    solo(player, vec![plain(0, 9)], |c| {
        assert_eq!(stats(c, 0), [(9, 1), (2, 8), (2, 8)]);
        assert!(c.sides[0].units[2].kw.contains(Keywords::DIVINE_SHIELD));
    });
}

#[test]
fn frostwolf_fervor_waits_for_round_seven_and_acts_once() {
    let early = band(vec![plain(3, 2)], vec![power("BG22_HERO_002p", true)], 6);
    solo(early, vec![plain(0, 9)], |c| {
        assert_eq!(stats(c, 0), [(3, 2)])
    });
    // Its first number at 0: it has acted this combat already.
    let used = band(vec![plain(3, 2)], vec![power("BG22_HERO_002p", false)], 8);
    solo(used, vec![plain(0, 9)], |c| {
        assert_eq!(stats(c, 0), [(3, 2)])
    });
}

#[test]
fn a_full_board_copies_once_a_death_makes_space_and_only_once() {
    let mut minions = vec![plain(1, 1); MAX_BOARD];
    minions[3] = plain(6, 6);
    let player = band(minions, vec![power("BG22_HERO_002p", true)], 7);
    solo(player, vec![plain(0, 9)], |c| {
        assert_eq!(c.sides[0].units.len(), MAX_BOARD);
        kill(c, 0, 0);
        assert_eq!(c.sides[0].units.len(), MAX_BOARD);
        assert_eq!(stats(c, 0)[2..4], [(6, 6), (6, 6)]);
        kill(c, 0, 0);
        assert_eq!(c.sides[0].units.len(), MAX_BOARD - 1);
    });
}

#[test]
fn lock_and_load_fires_the_held_minion_which_attacks_at_once() {
    let mut lnl = power("BG22_HERO_000p_Alt", true);
    lnl.held = Some(plain(4, 3));
    let player = band(vec![plain(1, 1)], vec![lnl.clone()], 7);
    solo(player, vec![plain(0, 9)], |c| {
        assert_eq!(stats(c, 1), [(0, 5)]);
        assert_eq!(stats(c, 0), [(1, 1), (4, 3)]);
    });
    // Nothing held (its first number at 0): nothing fired.
    let mut empty = lnl;
    empty.counters[0] = 0;
    let player = band(vec![plain(1, 1)], vec![empty], 7);
    solo(player, vec![plain(0, 9)], |c| {
        assert_eq!(stats(c, 1), [(0, 9)]);
        assert_eq!(c.sides[0].units.len(), 1);
    });
}

#[test]
fn rapid_reanimation_destroys_the_target_and_brings_back_its_exact_copy() {
    let mut rr = power("BG25_HERO_103p", false);
    rr.target = Some(1);
    let mut damaged = plain(5, 4);
    damaged.health = 3;
    let player = band(vec![plain(2, 2), damaged, plain(1, 1)], vec![rr], 3);
    let p = Prepared::new(&player, &band(vec![plain(0, 9)], Vec::new(), 3), true);
    let original = p.start[0][1].uid;
    let mut rng = Rng::new(5);
    let mut c = Combat::new(&p, &mut rng);
    c.start_of_combat(0, [true, true]);
    assert_eq!(stats(&c, 0), [(2, 2), (5, 3), (1, 1)]);
    assert_ne!(c.sides[0].units[1].uid, original);
    assert_eq!(c.sides[0].deaths, 1, "it died: its death counts");
}

#[test]
fn rapid_reanimations_copy_waits_while_the_board_is_full() {
    let mut rr = power("BG25_HERO_103p", false);
    rr.target = Some(2);
    let mut minions = vec![plain(1, 1); MAX_BOARD];
    minions[2] = plain(7, 7).with(Keywords::REBORN);
    let player = band(minions, vec![rr], 3);
    solo(player, vec![plain(0, 9)], |c| {
        // Its Reborn takes the place: the copy waits.
        assert_eq!(c.sides[0].units.len(), MAX_BOARD);
        assert_eq!(stats(c, 0)[2], (7, 1));
        kill(c, 0, 6);
        assert_eq!(stats(c, 0)[2], (7, 7));
        assert_eq!(c.sides[0].units.len(), MAX_BOARD);
    });
}

#[test]
fn a_named_minion_that_is_not_there_is_left_alone() {
    let mut rr = power("BG25_HERO_103p", false);
    rr.target = Some(5);
    let player = band(vec![plain(2, 2)], vec![rr], 3);
    solo(player, vec![plain(0, 9)], |c| {
        assert_eq!(stats(c, 0), [(2, 2)]);
        assert_eq!(c.sides[0].deaths, 0);
    });
}

#[test]
fn glorious_gloop_turns_the_target_into_the_teammates_highest_tier_minion() {
    let mut gloop = power("BGDUO_HERO_101p", false);
    gloop.target = Some(0);
    let first = band(vec![plain(1, 1), plain(2, 2)], vec![gloop], 5);
    let mate = band(
        vec![
            plain(9, 9),
            Minion::new("ZZ_TIER_FOUR", 4, 6).tier(4),
            Minion::new("ZZ_TIER_FOUR_B", 8, 8).tier(4),
        ],
        Vec::new(),
        5,
    );
    let opponent = Team::solo(band(vec![plain(0, 9)], Vec::new(), 5));
    after_start(Team::duo(first, mate), opponent, |c| {
        assert_eq!(stats(c, 0), [(4, 6), (2, 2)]);
        assert_eq!(ids(c, 0)[0], "ZZ_TIER_FOUR");
    });
}

#[test]
fn sandy_becomes_the_teammates_highest_health_minion() {
    let first = band(
        vec![Minion::new("BGDUO_125", 1, 1).tier(7), plain(1, 1)],
        Vec::new(),
        8,
    );
    let mate = band(
        vec![
            plain(3, 2),
            Minion::new("ZZ_BIG", 3, 9).with(Keywords::TAUNT),
            Minion::new("ZZ_BIG_TOO", 8, 9),
        ],
        Vec::new(),
        8,
    );
    let opponent = Team::solo(band(vec![plain(0, 9)], Vec::new(), 8));
    after_start(Team::duo(first, mate), opponent, |c| {
        assert_eq!(stats(c, 0), [(3, 9), (1, 1)]);
        assert_eq!(ids(c, 0)[0], "ZZ_BIG");
        assert!(c.sides[0].units[0].kw.contains(Keywords::TAUNT));
    });
}

#[test]
fn sandy_on_the_board_that_comes_in_copies_the_first_board_as_it_was_taken() {
    let first = band(vec![Minion::new("ZZ_FIRST", 6, 10)], Vec::new(), 8);
    let mate = band(vec![Minion::new("BGDUO_125", 1, 1).tier(7)], Vec::new(), 8);
    let opponent = Team::solo(band(vec![plain(0, 9)], Vec::new(), 8));
    after_start(Team::duo(first, mate), opponent, |c| {
        c.sides[0].units.clear();
        c.bring_in_teammates();
        assert_eq!(stats(c, 0), [(6, 10)]);
        assert_eq!(ids(c, 0), ["ZZ_FIRST"]);
    });
}

#[test]
fn stitched_salvager_destroys_its_left_neighbour_and_brings_back_its_copy() {
    let player = band(
        vec![Minion::new("ZZ_LEFT", 3, 3), Minion::new("BG31_999", 16, 4)],
        Vec::new(),
        8,
    );
    solo(player, vec![plain(0, 9)], |c| {
        assert_eq!(ids(c, 0), ["BG31_999"]);
        kill(c, 0, 0);
        assert_eq!(ids(c, 0), ["ZZ_LEFT"]);
        assert_eq!(stats(c, 0), [(3, 3)]);
    });
}

#[test]
fn a_golden_salvager_takes_both_neighbours_but_never_copies_a_salvager() {
    let player = band(
        vec![
            Minion::new("ZZ_LEFT", 3, 3),
            Minion::new("BG31_999_G", 32, 8),
            Minion::new("ZZ_RIGHT", 2, 5),
        ],
        Vec::new(),
        8,
    );
    solo(player, vec![plain(0, 9)], |c| {
        assert_eq!(ids(c, 0), ["BG31_999_G"]);
        kill(c, 0, 0);
        assert_eq!(ids(c, 0), ["ZZ_LEFT", "ZZ_RIGHT"]);
    });
    let player = band(
        vec![
            Minion::new("BG31_999", 16, 4),
            Minion::new("BG31_999", 16, 4),
        ],
        Vec::new(),
        8,
    );
    solo(player, vec![plain(0, 9)], |c| {
        assert_eq!(ids(c, 0), ["BG31_999"]);
        kill(c, 0, 0);
        assert!(c.sides[0].units.is_empty());
    });
}

#[test]
fn timewarped_chameleon_becomes_a_copy_of_its_left_neighbour() {
    let venom = Keywords::VENOMOUS.union(Keywords::DIVINE_SHIELD);
    let player = band(
        vec![
            Minion::new("ZZ_LEFT", 3, 12).with(venom),
            Minion::new("BG34_Giant_042", 9, 18),
        ],
        Vec::new(),
        8,
    );
    solo(player, vec![plain(0, 9)], |c| {
        assert_eq!(ids(c, 0), ["ZZ_LEFT", "ZZ_LEFT"]);
        assert_eq!(stats(c, 0), [(3, 12), (3, 12)]);
        assert_eq!(c.sides[0].units[1].kw, venom);
    });
    // The left-most one stays as it is.
    let alone = band(vec![Minion::new("BG34_Giant_042", 9, 18)], Vec::new(), 8);
    solo(alone, vec![plain(0, 9)], |c| {
        assert_eq!(ids(c, 0), ["BG34_Giant_042"])
    });
}

#[test]
fn a_board_taken_after_start_of_combat_waits_for_no_space() {
    // With start of combat off, the space powers are not played (their
    // effect at the start is already on the board).
    let mut minions = vec![plain(1, 1); MAX_BOARD];
    minions[0] = plain(6, 6);
    let player = band(minions, vec![power("BG22_HERO_002p", true)], 7);
    let opponent = band(vec![plain(0, 9)], Vec::new(), 7);
    let p = Prepared::new(&player, &opponent, false);
    let mut rng = Rng::new(5);
    let mut c = Combat::new(&p, &mut rng);
    c.open();
    kill(&mut c, 0, 1);
    assert_eq!(c.sides[0].units.len(), MAX_BOARD - 1);
}
