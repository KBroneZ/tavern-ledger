//! Hero powers and trinkets reviewed in T-631, on small boards whose outcome
//! is certain: Fragrant Phylactery's mark (given at the start of combat, or
//! shown on a board taken after it) and Eternal Portrait.

use super::{Combat, Prepared};
use crate::gifts::PHYLACTERY_MARK;
use crate::model::{Kit, KitCard, Minion, Tribes, Warband};
use crate::rng::Rng;

fn plain(attack: i32, health: i32) -> Minion {
    Minion::new("BGS_119", attack, health).tier(1)
}

fn band(minions: Vec<Minion>, powers: &[&str], trinkets: &[&str]) -> Warband {
    Warband {
        kit: Some(Kit {
            powers: powers.iter().map(|id| KitCard::new(*id)).collect(),
            trinkets: trinkets.iter().map(|id| KitCard::new(*id)).collect(),
            round: Some(7),
        }),
        tavern_tier: Some(1),
        ..Warband::new(minions)
    }
}

/// Prepares the combat (start of combat played when `soc`), lets `f` act on
/// it, and hands it over.
fn with<R>(player: Warband, soc: bool, f: impl FnOnce(&mut Combat) -> R) -> R {
    let opponent = band(vec![plain(0, 99)], &[], &[]);
    let p = Prepared::new(&player, &opponent, soc);
    let mut rng = Rng::new(7);
    let mut c = Combat::new(&p, &mut rng);
    if soc {
        c.start_of_combat(0, [true, true]);
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

const PHYLACTERY: &str = "BG20_HERO_282p";

#[test]
fn phylactery_marks_the_lowest_attack_minion_whose_death_gives_the_others_its_stats() {
    // The 2/5 has the lowest Attack: it dies, the others get +2/+5 (its full
    // Health, not what was left of it).
    let player = band(
        vec![plain(6, 1), plain(2, 5), plain(9, 9)],
        &[PHYLACTERY],
        &[],
    );
    with(player, true, |c| {
        c.sides[0].units[1].health = 3;
        kill(c, 1);
        assert_eq!(stats(c, 0), [(8, 6), (11, 14)]);
    });
}

#[test]
fn phylactery_picks_the_left_most_of_equal_attack() {
    let player = band(
        vec![plain(3, 1), plain(3, 4), plain(7, 7)],
        &[PHYLACTERY],
        &[],
    );
    with(player, true, |c| {
        // The right one of the two 3-Attack minions dies: nothing happens.
        kill(c, 1);
        assert_eq!(stats(c, 0), [(3, 1), (7, 7)]);
        kill(c, 0);
        assert_eq!(stats(c, 0), [(10, 8)]);
    });
}

#[test]
fn a_mark_shown_on_a_board_taken_after_start_of_combat_acts_the_same() {
    let marked = plain(4, 2).gift(PHYLACTERY_MARK);
    let player = band(vec![plain(1, 1), marked], &[PHYLACTERY], &[]);
    with(player, false, |c| {
        kill(c, 1);
        assert_eq!(stats(c, 0), [(5, 3)]);
    });
}

#[test]
fn without_start_of_combat_the_power_marks_nobody() {
    // A board taken after start of combat already shows the mark, or has
    // none: the power is not played again.
    let player = band(vec![plain(1, 1), plain(4, 2)], &[PHYLACTERY], &[]);
    with(player, false, |c| {
        kill(c, 0);
        assert_eq!(stats(c, 0), [(4, 2)]);
    });
}

const PORTRAIT: &str = "BG36_MagicItem_216";

fn knight() -> Minion {
    Minion::new("BG25_008", 4, 2).tribe(Tribes::UNDEAD).tier(2)
}

fn undead(attack: i32, health: i32) -> Minion {
    Minion::new("T631_UNDEAD", attack, health)
        .tribe(Tribes::UNDEAD)
        .tier(1)
}

#[test]
fn eternal_portrait_grows_every_knight_after_a_friendly_undead_dies() {
    let player = band(vec![undead(1, 1), knight(), plain(1, 1)], &[], &[PORTRAIT]);
    with(player, false, |c| {
        kill(c, 0);
        assert_eq!(stats(c, 0), [(8, 4), (1, 1)]);
        // A minion of another type changes nothing.
        kill(c, 1);
        assert_eq!(stats(c, 0), [(8, 4)]);
    });
}

#[test]
fn a_knight_that_dies_counts_for_both_the_knights_and_the_portrait() {
    let player = band(vec![knight(), knight()], &[], &[PORTRAIT]);
    with(player, false, |c| {
        kill(c, 0);
        // +4/+2 from the dead Knight's own text, +4/+2 from the Portrait.
        assert_eq!(stats(c, 0), [(12, 6)]);
    });
}

#[test]
fn without_the_portrait_an_undead_death_does_not_grow_the_knights() {
    let player = band(vec![undead(1, 1), knight()], &[], &[]);
    with(player, false, |c| {
        kill(c, 0);
        assert_eq!(stats(c, 0), [(4, 2)]);
    });
}

#[test]
fn a_knight_summoned_later_in_the_combat_comes_with_the_portraits_gains() {
    let player = band(vec![undead(1, 1), knight()], &[], &[PORTRAIT]);
    with(player, false, |c| {
        kill(c, 0);
        assert_eq!(c.game_stats(0, c.sides[0].units[0].card), (4, 2));
    });
}

#[test]
fn a_board_that_already_shows_the_mark_is_not_marked_again() {
    let marked = plain(4, 2).gift(PHYLACTERY_MARK);
    let player = band(vec![plain(1, 1), marked, plain(5, 5)], &[PHYLACTERY], &[]);
    with(player, true, |c| {
        // The 1/1 has the lowest Attack but gets no second mark.
        kill(c, 0);
        assert_eq!(stats(c, 0), [(4, 2), (5, 5)]);
        kill(c, 0);
        assert_eq!(stats(c, 0), [(9, 7)]);
    });
}

#[test]
fn bird_feeder_buffs_every_friendly_minion_after_two_deaths() {
    let player = band(
        vec![plain(1, 1), plain(1, 1), plain(2, 2)],
        &[],
        &["BG32_MagicItem_864t"],
    );
    with(player, false, |c| {
        kill(c, 0);
        assert_eq!(stats(c, 0), [(1, 1), (2, 2)]);
        kill(c, 0);
        assert_eq!(stats(c, 0), [(6, 6)]);
    });
}

#[test]
fn the_lesser_beetle_band_summons_one_beetle_with_taunt_after_five_deaths() {
    let minions = (0..6).map(|_| plain(1, 1)).collect();
    let player = band(minions, &[], &["BG32_MagicItem_860"]);
    with(player, false, |c| {
        for _ in 0..5 {
            kill(c, 0);
        }
        assert_eq!(c.sides[0].units.len(), 2);
        let beetle = &c.sides[0].units[1];
        assert_eq!(c.p.cards[beetle.card as usize].id, "BG28_603t");
        assert!(beetle.kw.contains(crate::model::Keywords::TAUNT));
    });
}
