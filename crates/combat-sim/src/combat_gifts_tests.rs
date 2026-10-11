//! Dark Gifts on small boards whose outcome is certain (T-527).

use super::{Combat, Prepared};
use crate::cards::Fidelity;
use crate::model::{Keywords, Minion, Tribes, Warband};
use crate::rng::Rng;
use crate::trials::{simulate, Options, Verdict};

const OFFENSIVE_SACRIFICE: &str = "BG36_MidGameEffect_000t";
const DEFENSIVE_SACRIFICE: &str = "BG36_MidGameEffect_000t2";
const RESISTANCE: &str = "BG36_MidGameEffect_000t7";
const ADMIRATION: &str = "BG36_MidGameEffect_000t9";
const PERSISTING_HORROR: &str = "BG36_MidGameEffect_000t12";
const HARPYS_TALONS: &str = "BG36_MidGameEffect_000t13";
const GILDING: &str = "BG36_MidGameEffect_000t14";
const TORETHS_BLESSING: &str = "BG36_MidGameEffect_000t15";
const JAWS_OF_DEATH: &str = "BG36_MidGameEffect_000t16";
const AMALGAMATION: &str = "BG36_MidGameEffect_000t22";
const INVULNERABILITY: &str = "BG36_MidGameEffect_000t60";
const GOLEMANCY: &str = "BG36_MidGameEffect_000t61";
const HOSTILITY: &str = "BG36_MidGameEffect_000t71";
const FORTITUDE: &str = "BG36_MidGameEffect_000t73";
const TRANSCENDENCE: &str = "BG36_MidGameEffect_000t81";
const WORSHIP: &str = "BG36_MidGameEffect_000t89";
const DOPPELGANGER: &str = "BG36_MidGameEffect_000t90";

fn band(minions: Vec<Minion>) -> Warband {
    Warband {
        minions,
        tavern_tier: Some(1),
    }
}

/// A keyword-only card, so the boards below are fully modelled.
fn plain(attack: i32, health: i32) -> Minion {
    Minion::new("BGS_119", attack, health).tier(1)
}

/// Bonehead: Deathrattle: summon two 1/1 Skeletons (golden: four).
fn bonehead() -> Minion {
    Minion::new("BG28_300", 1, 1).tribe(Tribes::UNDEAD).tier(1)
}

fn with_soc<R>(
    player: Vec<Minion>,
    opponent: Vec<Minion>,
    start_of_combat: bool,
    f: impl FnOnce(&mut Combat) -> R,
) -> R {
    let p = Prepared::new(&band(player), &band(opponent), start_of_combat);
    let mut rng = Rng::new(7);
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

/// Kills the minion at `ix` of side 0 (a hit from the first enemy) and
/// resolves the deaths.
fn kill(c: &mut Combat, ix: usize) {
    let (uid, by) = (uid_at(c, 0, ix), uid_at(c, 1, 0));
    let health = c.sides[0].units[ix].health;
    c.damage(0, uid, health, Keywords::NONE, by);
    c.process_deaths(1);
}

#[test]
fn offensive_sacrifice_gives_its_attack_to_a_friend_when_it_dies() {
    let gifted = plain(12, 1).gift(OFFENSIVE_SACRIFICE);
    with(vec![gifted, plain(1, 5)], vec![plain(0, 9)], |c| {
        kill(c, 0);
        assert_eq!(stats(c, 0), vec![(13, 5)]);
    });
}

#[test]
fn defensive_sacrifice_gives_its_full_health_not_what_is_left() {
    let gifted = plain(1, 12).gift(DEFENSIVE_SACRIFICE);
    with(vec![gifted, plain(1, 5)], vec![plain(0, 9)], |c| {
        let (uid, by) = (uid_at(c, 0, 0), uid_at(c, 1, 0));
        c.damage(0, uid, 4, Keywords::NONE, by);
        kill(c, 0);
        assert_eq!(stats(c, 0), vec![(1, 17)]);
    });
}

#[test]
fn start_of_combat_multipliers_double_or_triple_the_stats() {
    let board = vec![
        plain(3, 4).gift(RESISTANCE),
        plain(3, 4).gift(HOSTILITY),
        plain(3, 4).gift(TRANSCENDENCE),
    ];
    with_soc(board, vec![plain(0, 9)], true, |c| {
        c.start_of_combat(0, [true, false]);
        assert_eq!(stats(c, 0), vec![(3, 8), (6, 4), (9, 12)]);
    });
}

#[test]
fn admiration_gains_the_attack_of_the_minion_to_its_left() {
    let board = vec![
        plain(2, 2).gift(ADMIRATION),
        plain(5, 1),
        plain(1, 1).gift(ADMIRATION),
    ];
    with_soc(board, vec![plain(0, 9)], true, |c| {
        c.start_of_combat(0, [true, false]);
        // The left-most has no minion to its left.
        assert_eq!(stats(c, 0), vec![(2, 2), (5, 1), (6, 1)]);
    });
}

#[test]
fn jaws_of_death_triggers_the_deathrattle_at_start_of_combat() {
    let gifted = bonehead().gift(JAWS_OF_DEATH);
    with_soc(vec![gifted], vec![plain(0, 9)], true, |c| {
        c.start_of_combat(0, [true, false]);
        assert_eq!(ids(c, 0), ["BG28_300", "BG_ICC_026t", "BG_ICC_026t"]);
    });
}

#[test]
fn start_of_combat_gifts_wait_for_start_of_combat() {
    let board = vec![plain(3, 4).gift(HOSTILITY), bonehead().gift(JAWS_OF_DEATH)];
    with(board, vec![plain(0, 9)], |c| {
        // A logged board is taken after start of combat: nothing replays.
        c.process_deaths(0);
        assert_eq!(stats(c, 0), vec![(3, 4), (1, 1)]);
    });
}

#[test]
fn doppelganger_summons_one_copy_that_keeps_its_gifts() {
    let gifted = plain(4, 5).gift(DOPPELGANGER).gift(GOLEMANCY);
    with_soc(vec![gifted, plain(1, 1)], vec![plain(0, 9)], true, |c| {
        c.start_of_combat(0, [true, false]);
        assert_eq!(stats(c, 0), vec![(4, 5), (4, 5), (1, 1)]);
        let copy = &c.sides[0].units[1];
        assert_eq!(c.p.cards[copy.card as usize].gifts.len(), 2);
    });
}

#[test]
fn golemancy_summons_a_golem_with_the_minions_stats() {
    let gifted = plain(6, 7).gift(GOLEMANCY);
    with(vec![gifted], vec![plain(0, 9)], |c| {
        let (uid, by) = (uid_at(c, 0, 0), uid_at(c, 1, 0));
        c.damage(0, uid, 7, Keywords::NONE, by);
        c.process_deaths(1);
        assert_eq!(ids(c, 0), ["BG29_864t"]);
        assert_eq!(stats(c, 0), vec![(6, 7)]);
        // The Golem does not carry the gift on.
        let golem = &c.sides[0].units[0];
        assert!(c.p.cards[golem.card as usize].gifts.is_empty());
    });
}

#[test]
fn torets_blessing_shield_takes_three_hits() {
    let gifted = plain(1, 2)
        .with(Keywords::DIVINE_SHIELD)
        .gift(TORETHS_BLESSING);
    with(vec![gifted], vec![plain(0, 9)], |c| {
        let (uid, by) = (uid_at(c, 0, 0), uid_at(c, 1, 0));
        for _ in 0..2 {
            c.damage(0, uid, 5, Keywords::NONE, by);
            assert!(c.sides[0].units[0].kw.contains(Keywords::DIVINE_SHIELD));
        }
        c.damage(0, uid, 5, Keywords::NONE, by);
        assert!(!c.sides[0].units[0].kw.contains(Keywords::DIVINE_SHIELD));
        assert_eq!(stats(c, 0), vec![(1, 2)]);
    });
}

#[test]
fn invulnerability_takes_no_damage_while_attacking_only() {
    let gifted = plain(2, 3).gift(INVULNERABILITY);
    with(vec![gifted], vec![plain(5, 9)], |c| {
        c.swing(0, uid_at(c, 0, 0));
        assert_eq!(stats(c, 0), vec![(2, 3)]);
        assert_eq!(stats(c, 1), vec![(5, 7)]);
        c.swing(1, uid_at(c, 1, 0));
        assert!(c.sides[0].units.is_empty(), "it is hit when attacked");
    });
}

#[test]
fn persisting_horror_comes_back_whole_with_its_keywords() {
    let gifted = plain(4, 6).with(Keywords::TAUNT).gift(PERSISTING_HORROR);
    with(vec![gifted], vec![plain(0, 9)], |c| {
        assert!(c.sides[0].units[0].kw.contains(Keywords::REBORN));
        kill(c, 0);
        assert_eq!(stats(c, 0), vec![(4, 6)]);
        let kw = c.sides[0].units[0].kw;
        assert!(kw.contains(Keywords::TAUNT) && !kw.contains(Keywords::REBORN));
    });
}

#[test]
fn a_plain_reborn_comes_back_without_its_gifts() {
    let mut gifted = plain(4, 6).with(Keywords::REBORN).gift(GOLEMANCY);
    gifted.base = Some(crate::model::BaseCard {
        attack: 1,
        health: 1,
        keywords: Keywords::REBORN,
    });
    with(vec![gifted], vec![plain(0, 9)], |c| {
        kill(c, 0);
        // The Golem, then the minion back as a fresh 1/1 card with no gift
        // (the engine's order for a Deathrattle summon and Reborn).
        assert_eq!(ids(c, 0), ["BG29_864t", "BGS_119"]);
        let back = &c.sides[0].units[1];
        assert_eq!((back.attack, back.health), (1, 1));
        assert!(c.p.cards[back.card as usize].gifts.is_empty());
    });
}

#[test]
fn keyword_and_type_gifts_change_the_minion() {
    let board = vec![
        plain(1, 1).gift(HARPYS_TALONS),
        plain(1, 1).gift(AMALGAMATION),
        plain(1, 1).gift(FORTITUDE),
    ];
    with(board, vec![plain(0, 9)], |c| {
        let kw = c.sides[0].units[0].kw;
        assert!(kw.contains(Keywords::DIVINE_SHIELD) && kw.contains(Keywords::WINDFURY));
        let tribes = c.tribes(&c.sides[0].units[1]);
        assert!(tribes.is(Tribes::BEAST) && tribes.is(Tribes::MECH));
        assert_eq!(c.sides[0].units[2].kw, Keywords::NONE);
    });
}

#[test]
fn gilding_plays_the_golden_card() {
    let gilded = bonehead().gift(GILDING);
    with(vec![gilded], vec![plain(0, 9)], |c| {
        kill(c, 0);
        assert_eq!(ids(c, 0).len(), 4, "golden Bonehead summons four");
    });
}

#[test]
fn the_same_card_with_and_without_a_gift_plays_apart() {
    let board = vec![
        plain(5, 1).gift(OFFENSIVE_SACRIFICE),
        plain(5, 1),
        plain(1, 9),
    ];
    with(board, vec![plain(0, 9)], |c| {
        kill(c, 1);
        assert_eq!(
            stats(c, 0),
            vec![(5, 1), (1, 9)],
            "the plain one gives nothing"
        );
    });
}

fn opts(start_of_combat: bool) -> Options {
    Options {
        trials: 200,
        start_of_combat,
        ..Options::default()
    }
}

#[test]
fn a_gift_changes_the_odds() {
    let player = band(vec![plain(2, 4).gift(HOSTILITY)]);
    let opponent = band(vec![plain(3, 3)]);
    let Ok(Verdict::Odds(with_gift)) = simulate(&player, &opponent, &opts(true)) else {
        panic!("every card and gift is modelled");
    };
    assert_eq!(with_gift.won, with_gift.trials);
    assert!(with_gift.coverage.is_exact());
    let bare = band(vec![plain(2, 4)]);
    let Ok(Verdict::Odds(without)) = simulate(&bare, &opponent, &opts(true)) else {
        panic!("every card is modelled");
    };
    assert_eq!(without.tied, without.trials);
}

#[test]
fn an_unknown_gift_blocks_the_odds_by_name() {
    let player = band(vec![plain(2, 4).gift("BG99_NotAGift")]);
    let opponent = band(vec![plain(3, 3)]);
    let Ok(Verdict::NotModelled(cov)) = simulate(&player, &opponent, &opts(false)) else {
        panic!("an unknown gift is never ignored");
    };
    assert_eq!(cov.not_modelled, ["BG99_NotAGift"]);
}

#[test]
fn worship_blocks_only_when_start_of_combat_is_played() {
    let player = band(vec![plain(2, 4).gift(WORSHIP)]);
    let opponent = band(vec![plain(3, 3)]);
    assert!(matches!(
        simulate(&player, &opponent, &opts(false)),
        Ok(Verdict::Odds(_))
    ));
    let Ok(Verdict::NotModelled(cov)) = simulate(&player, &opponent, &opts(true)) else {
        panic!("the Deity's stats are unknown");
    };
    assert_eq!(cov.not_modelled, [WORSHIP]);
}

#[test]
fn approximate_gifts_are_named_with_the_reason() {
    let player = band(vec![plain(2, 4).gift(PERSISTING_HORROR)]);
    let opponent = band(vec![plain(3, 3)]);
    let Ok(Verdict::Odds(odds)) = simulate(&player, &opponent, &opts(false)) else {
        panic!("Persisting Horror plays");
    };
    let named: Vec<&str> = odds
        .coverage
        .approximate
        .iter()
        .map(|(id, _)| id.as_str())
        .collect();
    assert_eq!(named, [PERSISTING_HORROR]);
    let gift = crate::gifts::gift(PERSISTING_HORROR).unwrap();
    assert!(matches!(gift.fidelity, Fidelity::Approximate(_)));
}
