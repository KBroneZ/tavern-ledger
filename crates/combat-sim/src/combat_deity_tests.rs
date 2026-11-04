//! The Deities on hand-made boards (T-671).

use super::*;
use crate::model::{Deity, Tribes};
use crate::trials::Coverage;

const CTHUN: i32 = 130_610;
const YSHAARJ: i32 = 132_532;

fn aberration(attack: i32, health: i32) -> Minion {
    Minion::new("TEST_ABERRATION", attack, health)
        .tier(1)
        .tribe(Tribes::ABERRATION)
}

fn plain(attack: i32, health: i32) -> Minion {
    Minion::new("TEST_PLAIN", attack, health).tier(1)
}

fn band(minions: Vec<Minion>, deity: Deity) -> Warband {
    Warband {
        kit: None,
        minions,
        tavern_tier: Some(1),
        values: GameValues {
            deity,
            ..GameValues::default()
        },
    }
}

fn deity(dbf: i32, attack: i32, health: i32, needed: i32, progress: i32) -> Deity {
    Deity {
        dbf,
        attack,
        health,
        needed,
        progress,
    }
}

/// Kills the player's minions at `dead` (indexes), resolves the deaths and
/// gives the player's board as (card, Attack, Health).
fn after_deaths(player: Vec<Minion>, d: Deity, dead: &[usize]) -> Vec<(String, i32, i32)> {
    let p = Prepared::new(
        &band(player, d),
        &band(vec![plain(9, 9)], Deity::default()),
        false,
    );
    let mut rng = Rng::new(5);
    let mut c = Combat::new(&p, &mut rng);
    for &i in dead {
        c.sides[0].units[i].health = 0;
    }
    c.process_deaths(0);
    c.sides[0]
        .units
        .iter()
        .map(|u| (p.cards[u.card as usize].id.clone(), u.attack, u.health))
        .collect()
}

#[test]
fn c_thun_awakens_after_the_needed_deaths_and_splits_its_stats() {
    let board = vec![
        aberration(1, 1),
        aberration(1, 1),
        plain(1, 1),
        aberration(1, 1),
        aberration(1, 1),
        plain(1, 1),
    ];
    let out = after_deaths(board, deity(CTHUN, 10, 7, 4, 0), &[0, 1, 3, 4]);
    assert_eq!(out.len(), 3, "{out:?}");
    assert_eq!(out[2], ("BGFYM_000".to_string(), 10, 7), "at the right end");
    // 10/7 split between the two others: 5 Attack each, 3 Health each and
    // the extra one to one of them.
    let others = &out[..2];
    assert!(
        others
            .iter()
            .all(|(id, a, _)| id == "TEST_PLAIN" && *a == 6),
        "{out:?}"
    );
    let mut healths: Vec<i32> = others.iter().map(|(_, _, h)| *h).collect();
    healths.sort_unstable();
    assert_eq!(healths, [4, 5]);
}

#[test]
fn no_deity_awakens_while_too_few_died_or_it_is_unknown() {
    let board = || vec![aberration(1, 1), aberration(1, 1), plain(1, 1)];
    assert_eq!(
        after_deaths(board(), deity(CTHUN, 10, 7, 4, 0), &[0, 1]).len(),
        1
    );
    assert_eq!(after_deaths(board(), Deity::default(), &[0, 1]).len(), 1);
    // Deaths the game counted before the board was taken count too.
    let out = after_deaths(board(), deity(CTHUN, 10, 7, 4, 2), &[0, 1]);
    assert_eq!(out.last().map(|m| m.0.as_str()), Some("BGFYM_000"));
}

#[test]
fn a_deity_buff_before_it_awakens_goes_to_the_stats_it_awakens_with() {
    let sacrifice = Minion::new("BG36_113", 2, 1)
        .tier(3)
        .tribe(Tribes::ABERRATION);
    let board = vec![
        sacrifice,
        aberration(1, 1),
        aberration(1, 1),
        aberration(1, 1),
    ];
    let out = after_deaths(board, deity(CTHUN, 10, 7, 4, 0), &[0, 1, 2, 3]);
    // Nobody else on the board: C'Thun keeps its stats, +2/+1 from the
    // Deathrattle.
    assert_eq!(out, [("BGFYM_000".to_string(), 12, 8)]);
}

#[test]
fn y_shaarj_brings_back_the_first_aberrations_that_died() {
    let p = Prepared::new(
        &band(
            vec![aberration(3, 4), aberration(2, 2), plain(1, 1)],
            deity(YSHAARJ, 5, 5, 2, 0),
        ),
        &band(vec![plain(9, 9)], Deity::default()),
        false,
    );
    let mut rng = Rng::new(5);
    let mut c = Combat::new(&p, &mut rng);
    c.sides[0].units[0].health = 0;
    c.sides[0].units[1].health = 0;
    c.process_deaths(0);
    let ids = |c: &Combat| -> Vec<String> {
        c.sides[0]
            .units
            .iter()
            .map(|u| p.cards[u.card as usize].id.clone())
            .collect()
    };
    assert_eq!(ids(&c), ["TEST_PLAIN", "BGFYM_011"]);
    // Y'Shaarj dies: its Deathrattle brings both back where it was, with the
    // stats they died with; it is not counted itself.
    c.sides[0].units[1].health = 0;
    c.process_deaths(0);
    let back: Vec<(String, i32, i32)> = c.sides[0]
        .units
        .iter()
        .map(|u| (p.cards[u.card as usize].id.clone(), u.attack, u.health))
        .collect();
    assert_eq!(
        back,
        [
            ("TEST_PLAIN".to_string(), 1, 1),
            ("TEST_ABERRATION".to_string(), 3, 4),
            ("TEST_ABERRATION".to_string(), 2, 2),
        ]
    );
}

#[test]
fn a_deity_card_is_played_exactly_only_when_its_board_brings_the_deity() {
    let sacrifice = || {
        Minion::new("BG36_113", 2, 1)
            .tier(3)
            .tribe(Tribes::ABERRATION)
    };
    let theirs = band(vec![plain(1, 1)], Deity::default());
    let known = band(vec![sacrifice()], deity(CTHUN, 1, 1, 4, 0));
    let unknown = band(vec![sacrifice()], Deity::default());
    assert!(Coverage::of(&known, &theirs, false).approximate.is_empty());
    assert_eq!(Coverage::of(&unknown, &theirs, false).approximate.len(), 1);
}
