//! Rules fixed in the accuracy rounds (T-609), on small
//! boards whose outcome is certain.

use super::{Combat, Prepared};
use crate::combat::CombatResult;
use crate::model::{GameValues, Keywords, Minion, Team, Tribes, Warband};
use crate::rng::Rng;
use crate::trials::{simulate, Options, Verdict};

fn band(minions: Vec<Minion>) -> Warband {
    Warband {
        minions,
        tavern_tier: Some(1),
        values: Default::default(),
    }
}

fn plain(attack: i32, health: i32) -> Minion {
    Minion::new("TEST_PLAIN", attack, health).tier(1)
}

/// The only result of `player` against `opponent` over 200 trials.
fn only_result(player: Vec<Minion>, opponent: Vec<Minion>, start_of_combat: bool) -> CombatResult {
    let options = Options {
        trials: 200,
        start_of_combat,
        allow_unmodelled: true,
        ..Options::default()
    };
    let Ok(Verdict::Odds(o)) = simulate(&band(player), &band(opponent), &options) else {
        panic!("odds expected");
    };
    let results = [
        (CombatResult::Won, o.won),
        (CombatResult::Tie, o.tied),
        (CombatResult::Lost, o.lost),
    ];
    let seen: Vec<CombatResult> = results
        .iter()
        .filter(|(_, n)| *n > 0)
        .map(|(r, _)| *r)
        .collect();
    assert_eq!(seen.len(), 1, "one result expected: {o:?}");
    seen[0]
}

#[test]
fn heroic_broodmother_attacks_before_the_first_turn() {
    // Without its attack the opponent (two minions) goes first and kills it:
    // lost. With it, the Rally's Divine Shield takes the hit, it kills one
    // 1/1, then the other trades with it: a tie.
    let brood = Minion::new("BG36_849", 1, 1).tier(6);
    for soc in [false, true] {
        let r = only_result(vec![brood.clone()], vec![plain(1, 1), plain(1, 1)], soc);
        assert_eq!(r, CombatResult::Tie, "start of combat {soc}");
    }
}

#[test]
fn golden_heroic_broodmother_attacks_twice() {
    // Two swings, each with a fresh Divine Shield from the Rally: two of the
    // three 1/1s die before the opponent's first turn, the last one trades.
    let brood = Minion::new("BG36_849_G", 1, 1).tier(6);
    let r = only_result(
        vec![brood],
        vec![plain(1, 1), plain(1, 1), plain(1, 1)],
        false,
    );
    assert_eq!(r, CombatResult::Tie);
}

#[test]
fn heroic_broodmother_is_not_a_start_of_combat_gap() {
    // Played on every board, so it does not make shop-board odds approximate.
    assert!(!crate::cards::has_start_of_combat("BG36_849"));
    assert!(!crate::cards::has_start_of_combat("BG36_849_G"));
}

/// One swing of the player's minion at `ix` against a 0/50 that hits back
/// for nothing; the player's stats after it.
fn after_swing(player: Vec<Minion>, values: GameValues, ixs: &[usize]) -> Vec<(i32, i32)> {
    let band = Warband {
        values,
        ..band(player)
    };
    let p = Prepared::new(&band, &self::band(vec![plain(0, 50)]), false);
    let mut rng = Rng::new(1);
    let mut c = Combat::new(&p, &mut rng);
    for &ix in ixs {
        let uid = c.sides[0].units[ix].uid;
        c.swing(0, uid);
    }
    c.sides[0]
        .units
        .iter()
        .map(|u| (u.attack, u.health))
        .collect()
}

fn values(blood_gem: (i32, i32), spell: (i32, i32)) -> GameValues {
    GameValues {
        blood_gem,
        spell,
        ..GameValues::default()
    }
}

#[test]
fn blood_gems_give_the_sides_upgrades() {
    // Tusked Camper: Rally, a Blood Gem on itself; gems give +2/+1 more.
    let camper = Minion::new("BG33_886", 1, 10).tier(1);
    let got = after_swing(vec![camper], values((2, 1), (0, 0)), &[0]);
    assert_eq!(got, vec![(4, 12)]);
}

#[test]
fn without_values_blood_gems_give_one_one() {
    let camper = Minion::new("BG33_886_G", 1, 10).tier(1);
    let got = after_swing(vec![camper], GameValues::default(), &[0]);
    assert_eq!(got, vec![(3, 12)]);
}

#[test]
fn sanguine_refiner_upgrades_the_gems_played_after_it() {
    // Its Rally gives gems +1/+2 more; the Camper's gem then gives +2/+3.
    let refiner = Minion::new("BG33_885", 3, 8).tier(5);
    let camper = Minion::new("BG33_886", 1, 10).tier(1);
    let got = after_swing(vec![refiner, camper], GameValues::default(), &[0, 1]);
    assert_eq!(got, vec![(3, 8), (3, 13)]);
}

#[test]
fn prodigious_tusker_plays_upgraded_gems_on_the_attacker() {
    let tusker = Minion::new("BG33_430", 2, 5).tier(3);
    let got = after_swing(vec![tusker, plain(1, 10)], values((1, 1), (0, 0)), &[1]);
    assert_eq!(got, vec![(2, 5), (3, 12)]);
}

#[test]
fn bonker_plays_upgraded_gems_on_the_others() {
    let bonker = Minion::new("BG20_104", 2, 7).tier(4);
    let got = after_swing(vec![bonker, plain(1, 10)], values((1, 0), (0, 0)), &[0]);
    assert_eq!(got, vec![(2, 7), (3, 11)]);
}

#[test]
fn mighty_dragonbreath_buffs_get_the_spell_upgrades() {
    // Crimson Vindicator's Rally casts it: +3/+2 to every minion, again to
    // Dragons, again to those with Divine Shield; +1/+1 more each time.
    let vindicator = Minion::new("BG36_241", 8, 9).tribe(Tribes::DRAGON).tier(6);
    let got = after_swing(vec![vindicator, plain(1, 10)], values((0, 0), (1, 1)), &[0]);
    // The Vindicator: twice (all, Dragons); the plain minion: once.
    assert_eq!(got, vec![(16, 15), (5, 13)]);
}

/// The player's board after the minions at `ixs` die at once.
fn after_deaths(player: Vec<Minion>, values: GameValues, ixs: &[usize]) -> Vec<(String, i32, i32)> {
    let band = Warband {
        values,
        ..band(player)
    };
    let p = Prepared::new(&band, &self::band(vec![plain(0, 50)]), false);
    let mut rng = Rng::new(1);
    let mut c = Combat::new(&p, &mut rng);
    for &ix in ixs {
        c.sides[0].units[ix].health = 0;
    }
    c.process_deaths(0);
    c.sides[0]
        .units
        .iter()
        .map(|u| (c.p.cards[u.card as usize].id.clone(), u.attack, u.health))
        .collect()
}

fn knights_and_undead(knights_dead: i32, undead_attack: i32) -> GameValues {
    GameValues {
        knights_dead,
        undead_attack,
        ..GameValues::default()
    }
}

#[test]
fn a_summoned_eternal_knight_has_the_knights_that_died_and_the_undead_attack() {
    // Eternal Summoner: Reborn, Deathrattle: summon an Eternal Knight. Five
    // Knights died this game (+20/+10) and Undead have +71 Attack: the log's
    // 95/12 Knight (the user's game 4 round 10).
    let summoner = Minion::new("BG25_009", 8, 1).tribe(Tribes::UNDEAD).tier(5);
    let got = after_deaths(vec![summoner], knights_and_undead(5, 71), &[0]);
    assert!(got.contains(&("BG25_008".to_string(), 95, 12)), "{got:?}");
}

#[test]
fn an_eternal_knight_grows_when_another_dies() {
    let knight = || Minion::new("BG25_008", 8, 4).tribe(Tribes::UNDEAD).tier(2);
    let got = after_deaths(vec![knight(), knight()], GameValues::default(), &[0]);
    assert_eq!(got, vec![("BG25_008".to_string(), 12, 6)]);
}

#[test]
fn a_reborn_undead_gets_the_undead_attack_and_one_health() {
    // A 2/1 card with Reborn, 30/1 on the board: it comes back as a fresh
    // 2/1 with the side's +6 Undead Attack.
    let undead = Minion {
        base: Some(crate::model::BaseCard {
            attack: 2,
            health: 1,
            keywords: Keywords::REBORN,
        }),
        ..Minion::new("BG25_001", 30, 1)
            .with(Keywords::REBORN)
            .tribe(Tribes::UNDEAD)
            .tier(1)
    };
    let got = after_deaths(vec![undead], knights_and_undead(0, 6), &[0]);
    assert_eq!(got, vec![("BG25_001".to_string(), 8, 1)]);
}

#[test]
fn a_summoned_beetle_has_the_beetle_buffs() {
    // Buzzing Vermin: Deathrattle, summon a 2/2 Beetle.
    let vermin = Minion::new("BG31_803", 1, 1).tribe(Tribes::BEAST).tier(1);
    let values = GameValues {
        beetle: (12, 12),
        ..GameValues::default()
    };
    let got = after_deaths(vec![vermin], values, &[0]);
    assert_eq!(got, vec![("BG28_603t".to_string(), 14, 14)]);
}

#[test]
fn de_volition_ist_hits_the_highest_health_enemy_even_when_it_dies() {
    // The user's game 1 round 10: a 34/2 De-volition-ist attacks a 106/106,
    // dies, and its "after this attacks" still deals 34 to that minion.
    let devo = Minion::new("BG36_102", 34, 2).tier(5);
    let p = Prepared::new(&band(vec![devo]), &band(vec![plain(106, 106)]), false);
    let mut rng = Rng::new(1);
    let mut c = Combat::new(&p, &mut rng);
    let uid = c.sides[0].units[0].uid;
    c.swing(0, uid);
    assert!(c.sides[0].units.is_empty());
    assert_eq!(c.sides[1].units[0].health, 38);
}

#[test]
fn golden_persistent_poet_doubles_what_the_dragons_next_to_it_gain() {
    // Mighty Dragonbreath from a Crimson Vindicator: both Dragons next to
    // the golden Poet get each buff twice; the Poet itself once.
    let dragon = |id: &str, a, h| Minion::new(id, a, h).tribe(Tribes::DRAGON).tier(3);
    let got = after_swing(
        vec![
            dragon("BG36_241", 8, 9),
            dragon("BG29_813_G", 4, 6),
            dragon("TEST_PLAIN", 1, 10),
        ],
        GameValues::default(),
        &[0],
    );
    // +3/+2 to all, again to Dragons: +6/+4, doubled next to the Poet.
    assert_eq!(got, vec![(20, 17), (10, 10), (13, 18)]);
}

#[test]
fn a_teammates_board_comes_in_with_its_own_game_values() {
    // The first board's gems give +3/+3 more; the teammate's give none.
    let first = Warband {
        values: GameValues {
            blood_gem: (3, 3),
            ..GameValues::default()
        },
        ..band(vec![plain(1, 1)])
    };
    let second = band(vec![Minion::new("BG33_886", 1, 10).tier(1)]);
    let p = Prepared::teams(
        &Team::duo(first, second),
        &Team::solo(band(vec![plain(0, 50)])),
        false,
    );
    let mut rng = Rng::new(1);
    let mut c = Combat::new(&p, &mut rng);
    c.sides[0].units[0].health = 0;
    c.process_deaths(0);
    assert!(c.bring_in_teammates().is_some());
    let uid = c.sides[0].units[0].uid;
    c.swing(0, uid);
    assert_eq!(
        (c.sides[0].units[0].attack, c.sides[0].units[0].health),
        (2, 11)
    );
}

fn reasons(minions: Vec<Minion>, values: GameValues) -> Vec<(String, &'static str)> {
    let player = Warband {
        values,
        ..band(minions)
    };
    crate::trials::Coverage::of(&player, &band(vec![plain(1, 1)]), false).approximate
}

#[test]
fn blood_gem_and_spell_cards_are_exact_when_the_board_brings_their_upgrades() {
    // T-610: a board from the log or a saved round carries the upgrades; a
    // zero is unknown (an older record), so the card stays approximate.
    let gems = || vec![Minion::new("BG33_886", 3, 4).tier(1)];
    let spell = || vec![Minion::new("BG36_241", 4, 4).tier(4)];
    assert_eq!(reasons(gems(), GameValues::default()).len(), 1);
    assert_eq!(reasons(gems(), values((1, 0), (0, 0))), vec![]);
    assert_eq!(
        reasons(gems(), values((0, 0), (2, 2))).len(),
        1,
        "not its value"
    );
    assert_eq!(reasons(spell(), GameValues::default()).len(), 1);
    assert_eq!(reasons(spell(), values((0, 0), (0, 1))), vec![]);
    // The other side's upgrades are not this board's.
    let theirs = Warband {
        values: values((1, 1), (1, 1)),
        ..band(vec![plain(1, 1)])
    };
    let cov = crate::trials::Coverage::of(&band(gems()), &theirs, false);
    assert_eq!(cov.approximate.len(), 1);
}

#[test]
fn every_blood_gem_and_spell_upgrade_reason_is_the_shared_one() {
    // `played_with` (trials.rs) matches the reason itself: a card that words
    // the same gap its own way would stay approximate with its upgrades.
    let ids = crate::cards::EFFECTS
        .iter()
        .chain(crate::cards_round2::EFFECTS_2)
        .chain(crate::cards_round3::EFFECTS_3)
        .chain(crate::cards_round4::EFFECTS_4)
        .chain(crate::cards_round5::EFFECTS_5)
        .chain(crate::cards_round7::EFFECTS_7);
    for (id, rules) in ids {
        if let crate::cards::Fidelity::Approximate(why) = rules.fidelity {
            if why.contains("Blood Gem") && why.contains("upgrade") {
                assert_eq!(why, crate::cards_round1::BLOOD_GEM, "{id}");
            }
            if why.contains("spell") && why.contains("grow") {
                assert_eq!(why, crate::cards_round2::SPELL, "{id}");
            }
        }
    }
}
