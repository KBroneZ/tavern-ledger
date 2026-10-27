//! Hand-made logs: one that follows the rules, and one per rule it breaks.

use super::*;
use crate::model::Tribes;

const PLAIN: &str = "TEST_PLAIN";

fn m(attack: i32, health: i32) -> Minion {
    Minion::new(PLAIN, attack, health).tier(1)
}

fn band(minions: &[Minion]) -> Warband {
    Warband {
        kit: None,
        minions: minions.to_vec(),
        tavern_tier: Some(1),
        values: Default::default(),
    }
}

fn step(
    attacker: (usize, usize),
    defender: usize,
    player: &[Minion],
    opponent: &[Minion],
) -> LoggedStep {
    LoggedStep {
        attacker,
        defender: Some(defender),
        before: [player.to_vec(), opponent.to_vec()],
        all_keywords: true,
    }
}

fn run(steps: &[LoggedStep], result: CombatResult) -> Replay {
    let [p, o] = &steps[0].before;
    replay(&band(p), &band(o), steps, Span::Whole(Some(result)), 1)
}

#[test]
fn a_log_that_follows_the_rules_replays_to_the_end() {
    // 2/3 and 1/1 against a 1/2: the player has more minions and goes first.
    let p0 = [m(2, 3), m(1, 1)];
    let o0 = [m(1, 2)];
    let steps = [step((0, 0), 0, &p0, &o0)];
    let r = run(&steps, CombatResult::Won);
    assert_eq!(r.difference, None, "{r:?}");
    assert_eq!((r.attacks, r.matched), (1, 1));
}

#[test]
fn the_side_with_fewer_minions_attacking_first_is_named() {
    let p0 = [m(2, 3), m(1, 1)];
    let o0 = [m(1, 2)];
    let steps = [step((1, 0), 0, &p0, &o0)];
    let d = run(&steps, CombatResult::Won).difference.expect("differs");
    assert_eq!((d.attack, d.rule), (1, Rule::FirstAttacker));
}

#[test]
fn equal_counts_take_the_logs_coin_flip() {
    let p0 = [m(1, 5)];
    let o0 = [m(1, 5)];
    let p1 = [m(1, 4)];
    let o1 = [m(1, 4)];
    let steps = [step((1, 0), 0, &p0, &o0), step((0, 0), 0, &p1, &o1)];
    let r = run(&steps, CombatResult::Tie);
    // Both attacked once; the engine would go on.
    let d = r.difference.expect("the log stops early");
    assert_eq!((r.matched, d.rule), (2, Rule::End));
}

#[test]
fn hitting_past_a_taunt_is_a_target_difference() {
    let p0 = [m(3, 3), m(1, 1)];
    let o0 = [m(1, 1), m(1, 1).with(Keywords::TAUNT)];
    let steps = [step((0, 0), 0, &p0, &o0)];
    let d = run(&steps, CombatResult::Won).difference.expect("differs");
    assert_eq!(d.rule, Rule::Target);
    assert!(d.detail.contains("position 1"), "{}", d.detail);
}

#[test]
fn the_wrong_minion_attacking_is_an_attack_order_difference() {
    let p0 = [m(1, 9), m(1, 9)];
    let o0 = [m(1, 9)];
    let steps = [step((0, 1), 0, &p0, &o0)];
    let d = run(&steps, CombatResult::Tie).difference.expect("differs");
    assert_eq!(d.rule, Rule::AttackOrder);
}

#[test]
fn a_board_unlike_the_engines_after_an_attack_is_named() {
    let p0 = [m(2, 3), m(1, 1)];
    let o0 = [m(1, 2)];
    // The log claims the 1/2 survived the 2-damage hit with 1 Health.
    let p1 = [m(2, 2), m(1, 1)];
    let o1 = [m(1, 1)];
    let steps = [step((0, 0), 0, &p0, &o0), step((1, 0), 0, &p1, &o1)];
    let d = run(&steps, CombatResult::Won).difference.expect("differs");
    assert_eq!((d.attack, d.rule), (2, Rule::Minions));

    let o1 = [m(1, 2)];
    let p1 = [m(2, 3), m(1, 1)];
    let steps = [step((0, 0), 0, &p0, &o0), step((1, 0), 0, &p1, &o1)];
    // Same minions, other numbers.
    let o0 = [m(1, 5)];
    let steps2 = [
        step((0, 0), 0, &p0, &o0),
        step((1, 0), 0, &[m(2, 2), m(1, 1)], &[m(1, 4)]),
    ];
    assert_eq!(
        run(&steps, CombatResult::Won).difference.unwrap().rule,
        Rule::Minions
    );
    let d = run(&steps2, CombatResult::Lost)
        .difference
        .expect("differs");
    assert_eq!((d.attack, d.rule), (2, Rule::Stats));
}

#[test]
fn windfury_attacks_twice_before_the_other_side() {
    let wf = m(1, 9).with(Keywords::WINDFURY);
    let p0 = [wf.clone(), m(1, 9)];
    let o0 = [m(0, 9)];
    let p1 = p0.clone();
    let o1 = [m(0, 8)];
    let steps = [step((0, 0), 0, &p0, &o0), step((0, 0), 0, &p1, &o1)];
    let r = run(&steps, CombatResult::Tie);
    assert_eq!(r.matched, 2, "{r:?}");
    // A third attack by the same minion breaks the order.
    let o2 = [m(0, 7)];
    let steps = [
        step((0, 0), 0, &p0, &o0),
        step((0, 0), 0, &p1, &o1),
        step((0, 0), 0, &p1, &o2),
    ];
    let d = run(&steps, CombatResult::Tie).difference.expect("differs");
    assert_eq!((d.attack, d.rule), (3, Rule::AttackOrder));
}

#[test]
fn a_missing_defender_stops_the_replay() {
    let p0 = [m(2, 3)];
    let o0 = [m(1, 2)];
    let mut s = step((0, 0), 0, &p0, &o0);
    s.defender = None;
    let steps = [s];
    let r = run(&steps, CombatResult::Won);
    assert_eq!(r.difference.map(|d| d.rule), Some(Rule::NoDefender));
    assert_eq!(r.matched, 0);
}

#[test]
fn a_result_the_game_disagrees_with_is_an_end_difference() {
    let p0 = [m(2, 3), m(1, 1)];
    let o0 = [m(1, 2)];
    let steps = [step((0, 0), 0, &p0, &o0)];
    let d = run(&steps, CombatResult::Lost).difference.expect("differs");
    assert_eq!(d.rule, Rule::End);
}

#[test]
fn a_duos_first_leg_stops_when_a_board_is_gone() {
    let p0 = [m(2, 3), m(1, 1)];
    let o0 = [m(1, 2)];
    // The teammate's board comes in after: not the engine's to play.
    let teammate = [m(5, 5)];
    let steps = [step((0, 0), 0, &p0, &o0), step((1, 0), 0, &p0, &teammate)];
    let r = replay(&band(&p0), &band(&o0), &steps, Span::FirstLeg, 1);
    assert_eq!((r.matched, r.difference), (1, None));
}

#[test]
fn a_duos_combat_replays_through_the_teammates_board() {
    // The two 1/1s trade; both teammates' boards (2/5 and 1/3) come in at
    // once with one minion each: a coin flip, which the log's side (the
    // player) is taken to have won.
    let mate = [m(2, 5)];
    let foe_mate = [m(1, 3)];
    let player = Team::duo(band(&[m(1, 1)]), band(&mate));
    let opponent = Team::duo(band(&[m(1, 1)]), band(&foe_mate));
    let steps = [
        step((0, 0), 0, &[m(1, 1)], &[m(1, 1)]),
        step((0, 0), 0, &mate, &foe_mate),
        step((1, 0), 0, &[m(2, 4)], &[m(1, 1)]),
    ];
    let r = replay_teams(
        &player,
        &opponent,
        &steps,
        Span::Whole(Some(CombatResult::Won)),
        1,
    );
    assert_eq!(r.difference, None, "{r:?}");
    assert_eq!((r.matched, r.boards_in), (3, 2));
}

#[test]
fn the_wrong_side_after_a_teammate_comes_in_is_named() {
    // The player's board is gone; their teammate brings two minions against
    // one: the teammate's side attacks first, not the opponent.
    let mate = [m(1, 5), m(1, 5)];
    let player = Team::duo(band(&[m(1, 1)]), band(&mate));
    let opponent = Team::solo(band(&[m(1, 9)]));
    let steps = [
        step((0, 0), 0, &[m(1, 1)], &[m(1, 9)]),
        step((1, 0), 0, &mate, &[m(1, 8)]),
    ];
    let r = replay_teams(&player, &opponent, &steps, Span::Whole(None), 1);
    let d = r.difference.expect("differs");
    assert_eq!((d.attack, d.rule), (2, Rule::AfterTeammate));
    assert_eq!(r.boards_in, 1);
}

#[test]
fn a_start_of_combat_attack_comes_before_the_first_turn() {
    // Heroic Broodmother attacks first though the opponent has more minions;
    // then the opponent takes the first turn, and the player's turn starts
    // from its left-most minion (the start-of-combat swing is not a turn).
    let brood = Minion::new("BG36_849", 3, 7).tier(6);
    let p0 = [m(1, 3), brood.clone()];
    let o0 = [m(1, 5), m(1, 5), m(1, 5)];
    // The Rally's Divine Shield takes the hit back.
    let o1 = [m(1, 2), m(1, 5), m(1, 5)];
    let p2 = [m(1, 2), brood.clone()];
    let o2 = [m(1, 1), m(1, 5), m(1, 5)];
    let steps = [
        step((0, 1), 0, &p0, &o0),
        step((1, 0), 0, &p0, &o1),
        step((0, 0), 1, &p2, &o2),
    ];
    let r = replay(&band(&p0), &band(&o0), &steps, Span::FirstLeg, 1);
    assert_eq!(r.difference, None, "{r:?}");
    assert_eq!(r.matched, 3);
}

#[test]
fn a_normal_first_attack_before_the_start_of_combat_one_is_named() {
    let brood = Minion::new("BG36_849", 3, 7).tier(6);
    let p0 = [m(1, 3), brood];
    let o0 = [m(1, 5), m(1, 5), m(1, 5)];
    let steps = [step((1, 0), 0, &p0, &o0)];
    let d = replay(&band(&p0), &band(&o0), &steps, Span::FirstLeg, 1)
        .difference
        .expect("differs");
    assert_eq!((d.attack, d.rule), (1, Rule::FirstAttacker));
}

#[test]
fn a_difference_names_the_card_it_is_about() {
    let other = |a, h| Minion::new("TEST_OTHER", a, h).tier(1);
    // The log's attacker when the order differs.
    let p0 = [m(1, 9), other(1, 9)];
    let o0 = [m(1, 9)];
    let d = run(&[step((0, 1), 0, &p0, &o0)], CombatResult::Tie)
        .difference
        .expect("differs");
    assert_eq!(d.card.as_deref(), Some("TEST_OTHER"));
    // The target the engine does not allow.
    let p0 = [m(3, 3), m(1, 1)];
    let o0 = [other(1, 1), m(1, 1).with(Keywords::TAUNT)];
    let d = run(&[step((0, 0), 0, &p0, &o0)], CombatResult::Won)
        .difference
        .expect("differs");
    assert_eq!(d.card.as_deref(), Some("TEST_OTHER"));
    // The minion whose numbers differ.
    let p0 = [m(2, 3), m(1, 1)];
    let o0 = [other(1, 5)];
    let steps = [
        step((0, 0), 0, &p0, &o0),
        step((1, 0), 0, &[m(2, 2), m(1, 1)], &[other(1, 4)]),
    ];
    let d = run(&steps, CombatResult::Lost).difference.expect("differs");
    assert_eq!(
        (d.rule, d.card.as_deref()),
        (Rule::Stats, Some("TEST_OTHER"))
    );
    // Who attacks first: the log's attacker.
    let p0 = [m(2, 3), m(1, 1)];
    let o0 = [other(1, 2)];
    let d = run(&[step((1, 0), 0, &p0, &o0)], CombatResult::Won)
        .difference
        .expect("differs");
    assert_eq!(d.rule, Rule::FirstAttacker);
    assert_eq!(d.card.as_deref(), Some("TEST_OTHER"));
    // The end of combat is about no card.
    let p0 = [m(1, 5)];
    let o0 = [m(1, 5)];
    let d = run(&[step((1, 0), 0, &p0, &o0)], CombatResult::Tie)
        .difference
        .expect("the log stops early");
    assert_eq!((d.rule, d.card), (Rule::End, None));
}

/// A card whose Deathrattle gives a random friendly Undead +1/+2
/// (`BG25_022`, played exactly).
const RANDOM_BUFF: &str = "BG25_022";

/// The player's random-buff 1/1 and two Undead 1/5s against a 5/10: the
/// 1/1 attacks, dies and buffs one of the two; the log says which.
fn random_buff_log(buffed: usize) -> [LoggedStep; 2] {
    let undead = |a, h| m(a, h).tribe(Tribes::UNDEAD);
    let p0 = [
        Minion::new(RANDOM_BUFF, 1, 1).tier(2),
        undead(1, 5),
        undead(1, 5),
    ];
    let o0 = [m(5, 10)];
    let mut p1 = vec![undead(1, 5), undead(1, 5)];
    p1[buffed] = undead(2, 7);
    let o1 = [m(5, 9)];
    // The opponent's 5/9 then hits the first Undead.
    [step((0, 0), 0, &p0, &o0), step((1, 0), 0, &p1, &o1)]
}

#[test]
fn a_random_buff_the_log_gave_elsewhere_is_rerolled_not_a_rule() {
    for buffed in [0, 1] {
        let steps = random_buff_log(buffed);
        let [p, o] = &steps[0].before;
        let r = replay(&band(p), &band(o), &steps, Span::FirstLeg, 1);
        assert_eq!(r.difference, None, "buffed {buffed}: {r:?}");
        assert_eq!(r.matched, 2);
        assert!(r.rerolled <= 1);
    }
    // One of the two logs takes another roll than the first one.
    let rolled: usize = [0, 1]
        .map(|b| {
            let steps = random_buff_log(b);
            let [p, o] = &steps[0].before;
            replay(&band(p), &band(o), &steps, Span::FirstLeg, 1).rerolled
        })
        .iter()
        .sum();
    assert_eq!(rolled, 1);
}

#[test]
fn a_difference_no_roll_explains_is_still_a_rule() {
    let mut steps = random_buff_log(0);
    // Neither Undead could end at 3/7: no roll gives that.
    steps[1].before[0][0] = m(3, 7).tribe(Tribes::UNDEAD);
    let [p, o] = &steps[0].before;
    let r = replay(&band(p), &band(o), &steps, Span::FirstLeg, 1);
    let d = r.difference.expect("differs");
    assert_eq!((d.attack, d.rule, r.rerolled), (2, Rule::Stats, 0));
}

#[test]
fn rerolls_give_the_same_replay_on_every_run() {
    let steps = random_buff_log(1);
    let [p, o] = &steps[0].before;
    let a = replay(&band(p), &band(o), &steps, Span::FirstLeg, 7);
    let b = replay(&band(p), &band(o), &steps, Span::FirstLeg, 7);
    assert_eq!(a, b);
}
