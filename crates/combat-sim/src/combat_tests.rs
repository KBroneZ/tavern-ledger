//! Small hand-built boards whose outcome is certain (or whose odds follow
//! from one coin flip).

use super::*;
use crate::model::Minion;
use crate::trials::{simulate, Odds, Options, Verdict};

const PLAIN: &str = "TEST_PLAIN";

fn plain(attack: i32, health: i32) -> Minion {
    Minion::new(PLAIN, attack, health).tier(1)
}

fn band(minions: Vec<Minion>) -> Warband {
    Warband {
        kit: None,
        minions,
        tavern_tier: Some(1),
        values: Default::default(),
    }
}

fn odds(player: Vec<Minion>, opponent: Vec<Minion>) -> Odds {
    let opts = Options {
        trials: 2_000,
        seed: 11,
        start_of_combat: false,
        allow_unmodelled: true,
    };
    match simulate(&band(player), &band(opponent), &opts).unwrap() {
        Verdict::Odds(o) => o,
        Verdict::NotModelled(c) => panic!("not modelled: {c:?}"),
    }
}

fn always(o: &Odds, result: CombatResult) {
    assert_eq!(o.share(result), 1.0, "{o:?}");
}

#[test]
fn empty_boards_tie_with_no_damage() {
    let o = odds(vec![], vec![]);
    always(&o, CombatResult::Tie);
    assert!(o.damage_dealt.is_empty() && o.damage_taken.is_empty());
}

#[test]
fn a_board_against_nothing_wins_and_deals_tiers() {
    let o = odds(vec![plain(1, 1), plain(1, 1)], vec![]);
    always(&o, CombatResult::Won);
    // Two tier 1 minions + tavern tier 1.
    assert_eq!(o.damage_dealt.get(&3), Some(&o.trials));
}

#[test]
fn unknown_tier_means_unknown_damage_not_zero() {
    let player = band(vec![Minion::new(PLAIN, 3, 3)]);
    let opts = Options {
        allow_unmodelled: true,
        trials: 10,
        ..Options::default()
    };
    let Verdict::Odds(o) = simulate(&player, &band(vec![]), &opts).unwrap() else {
        panic!()
    };
    assert_eq!(o.won, 10);
    assert_eq!(o.damage_unknown, 10);
    assert!(o.damage_dealt.is_empty());
}

#[test]
fn bigger_minion_always_wins() {
    always(
        &odds(vec![plain(2, 2)], vec![plain(1, 1)]),
        CombatResult::Won,
    );
    always(
        &odds(vec![plain(1, 1)], vec![plain(2, 2)]),
        CombatResult::Lost,
    );
}

#[test]
fn equal_trade_ties() {
    always(
        &odds(vec![plain(1, 1)], vec![plain(1, 1)]),
        CombatResult::Tie,
    );
}

#[test]
fn zero_attack_on_both_sides_is_a_tie_not_a_hang() {
    let o = odds(vec![plain(0, 5)], vec![plain(0, 5)]);
    always(&o, CombatResult::Tie);
    assert_eq!(o.stalled, 0);
}

#[test]
fn the_side_with_more_minions_attacks_first() {
    // Player: 3/1 and 0/1; opponent: one 3/1. The player's 3/1 attacks first
    // and trades; the 0/1 is left: a win every time. If the opponent went
    // first it could hit the 0/1 and win.
    let o = odds(vec![plain(3, 1), plain(0, 1)], vec![plain(3, 1)]);
    always(&o, CombatResult::Won);
}

#[test]
fn equal_counts_flip_a_coin_for_who_goes_first() {
    // Each side has a 1/1 (left) and a 5/5. Whoever goes first loses its 1/1
    // or kills the other's 1/1; the odds come from the coin and targets.
    let o = odds(
        vec![plain(1, 1), plain(5, 5)],
        vec![plain(1, 1), plain(5, 5)],
    );
    // Symmetric boards: wins and losses are equally likely.
    let diff = (o.share(CombatResult::Won) - o.share(CombatResult::Lost)).abs();
    assert!(diff < 0.06, "{o:?}");
}

#[test]
fn divine_shield_absorbs_one_hit() {
    let shielded = plain(1, 1).with(Keywords::DIVINE_SHIELD);
    always(&odds(vec![shielded], vec![plain(1, 1)]), CombatResult::Won);
}

/// Runs `f` on a fresh combat between the two boards (uids: the player's
/// minions from 1, then the opponent's).
fn with_combat<R>(
    player: Vec<Minion>,
    opponent: Vec<Minion>,
    f: impl FnOnce(&mut Combat) -> R,
) -> R {
    let p = Prepared::new(&band(player), &band(opponent), false);
    let mut rng = Rng::new(21);
    let mut c = Combat {
        p: &p,
        rng: &mut rng,
        sides: [
            SideState {
                units: p.start[0].clone(),
                next: 0,
                values: Default::default(),
                deaths: 0,
            },
            SideState {
                units: p.start[1].clone(),
                next: 0,
                values: Default::default(),
                deaths: 0,
            },
        ],
        next_uid: 100,
        forced_target: None,
        legs: Legs::of(&p),
    };
    f(&mut c)
}

fn healths(c: &Combat, side: usize) -> Vec<i32> {
    c.sides[side].units.iter().map(|u| u.health).collect()
}

#[test]
fn taunt_must_be_attacked_first() {
    let opp = vec![plain(0, 5), plain(0, 5).with(Keywords::TAUNT), plain(0, 5)];
    with_combat(vec![plain(1, 1)], opp, |c| {
        for _ in 0..30 {
            assert_eq!(c.choose_target(1), Some(3));
        }
    });
}

#[test]
fn stealth_is_not_targeted_while_others_are_open() {
    let opp = vec![plain(0, 1).with(Keywords::STEALTH), plain(0, 1)];
    with_combat(vec![plain(1, 1)], opp.clone(), |c| {
        for _ in 0..30 {
            assert_eq!(c.choose_target(1), Some(3));
        }
    });
    // All Stealthed: any of them.
    let hidden = vec![plain(0, 1).with(Keywords::STEALTH); 2];
    with_combat(vec![plain(1, 1)], hidden, |c| {
        let picks: std::collections::HashSet<_> = (0..30).map(|_| c.choose_target(1)).collect();
        assert_eq!(picks.len(), 2);
    });
}

#[test]
fn windfury_attacks_twice_in_one_turn_and_mega_windfury_four_times() {
    let opp = || vec![plain(0, 1); 5];
    for (kw, left) in [
        (Keywords::NONE, 4),
        (Keywords::WINDFURY, 3),
        (Keywords::MEGA_WINDFURY, 1),
    ] {
        with_combat(vec![plain(1, 50).with(kw)], opp(), |c| {
            assert!(c.attack_turn(0));
            assert_eq!(c.sides[1].units.len(), left, "{kw:?}");
        });
    }
}

#[test]
fn cleave_hits_the_target_and_its_neighbours() {
    // The Taunt in the middle is the target; both neighbours take 3 too.
    let collector = Minion::new("BG26_817", 3, 9).tier(4);
    let opp = vec![
        plain(0, 5),
        plain(0, 5).with(Keywords::TAUNT),
        plain(0, 5),
        plain(0, 5),
    ];
    with_combat(vec![collector], opp, |c| {
        c.swing(0, 1);
        assert_eq!(healths(c, 1), [2, 2, 2, 5]);
    });
    // A plain minion hits only the target.
    let opp = vec![plain(0, 5), plain(0, 5).with(Keywords::TAUNT), plain(0, 5)];
    with_combat(vec![plain(3, 9)], opp, |c| {
        c.swing(0, 1);
        assert_eq!(healths(c, 1), [5, 2, 5]);
    });
}

#[test]
fn a_token_already_on_the_board_is_still_summoned_with_its_own_stats() {
    // The opponent has a Skeleton with unknown card facts; the player's
    // Bonehead must still summon two 1/1 Skeletons.
    let bonehead = Minion::new("BG28_300", 1, 1);
    let skeleton = Minion::new("BG_ICC_026t", 1, 1);
    with_combat(vec![bonehead], vec![skeleton], |c| {
        c.sides[0].units[0].health = 0;
        c.process_deaths(0);
        let stats: Vec<(i32, i32)> = c.sides[0]
            .units
            .iter()
            .map(|u| (u.attack, u.health))
            .collect();
        assert_eq!(stats, [(1, 1), (1, 1)]);
    });
}

#[test]
fn huge_stats_saturate_instead_of_overflowing() {
    let big = Minion::new("BG36_102_G", i32::MAX - 1, 10).tier(5);
    with_combat(vec![big], vec![plain(0, 5), plain(0, 5)], |c| {
        c.swing(0, 1);
        assert!(c.sides[1].units.is_empty());
    });
}

#[test]
fn a_card_that_gives_reborn_flags_minions_without_card_stats() {
    let mummifier = Minion::new("BG28_309", 5, 2).tribe(Tribes::UNDEAD);
    let undead = Minion::new("BG_ICC_026t", 1, 1).tribe(Tribes::UNDEAD);
    let cov = crate::trials::Coverage::of(&band(vec![mummifier, undead]), &band(vec![]), false);
    assert!(
        cov.approximate.iter().any(|(id, _)| id == "BG_ICC_026t"),
        "{cov:?}"
    );
}

#[test]
fn venomous_kills_any_size_but_not_through_divine_shield() {
    let venom = plain(1, 1).with(Keywords::VENOMOUS);
    always(
        &odds(vec![venom.clone()], vec![plain(1, 100)]),
        CombatResult::Tie,
    );
    let shielded = plain(1, 100).with(Keywords::DIVINE_SHIELD);
    always(&odds(vec![venom], vec![shielded]), CombatResult::Lost);
}

#[test]
fn venomous_is_used_up_but_poisonous_is_not() {
    // A 1/10 Venomous against two 1/30s: kills one, then must chip the
    // other: it cannot win. Poisonous kills both.
    let opp = || vec![plain(1, 30), plain(1, 30)];
    let venom = odds(vec![plain(1, 10).with(Keywords::VENOMOUS)], opp());
    assert_eq!(venom.won, 0, "{venom:?}");
    let poison = odds(vec![plain(1, 10).with(Keywords::POISONOUS)], opp());
    always(&poison, CombatResult::Won);
}

#[test]
fn reborn_comes_back_once_with_one_health_and_base_attack() {
    let mut reborn = plain(5, 1).with(Keywords::REBORN);
    reborn.base = Some(BaseCard {
        attack: 2,
        health: 3,
        keywords: Keywords::REBORN,
    });
    // Against a 1/3: the 5/1 kills it while dying: tie without Reborn; with
    // Reborn the 2/1 copy stays: a win.
    always(
        &odds(vec![reborn.clone()], vec![plain(1, 3)]),
        CombatResult::Won,
    );
    // Against two 1/5s it trades into the first, the 2/1 copy cannot finish
    // the second: a loss, which shows the copy has 2 Attack, not 5.
    let o = odds(vec![reborn], vec![plain(1, 5), plain(1, 5)]);
    always(&o, CombatResult::Lost);
}

#[test]
fn deathrattle_summons_where_the_minion_died() {
    // Harmless Bonehead (Deathrattle: two 1/1 Skeletons) against a 1/2. The
    // trade leaves a 1/1 against two Skeletons, which always win; without
    // the Deathrattle it would be a loss. Against a 1/3 it is always a tie.
    let bonehead = || Minion::new("BG28_300", 1, 1).tier(1);
    always(
        &odds(vec![bonehead()], vec![plain(1, 2)]),
        CombatResult::Won,
    );
    always(
        &odds(vec![bonehead()], vec![plain(1, 3)]),
        CombatResult::Tie,
    );
}

#[test]
fn summons_stop_at_seven_minions() {
    // Six 0/1s and a Bonehead that dies: only one Skeleton fits.
    let mut player = vec![Minion::new("BG28_300", 1, 1).tier(1)];
    player.extend((0..6).map(|_| plain(0, 1)));
    let p = Prepared::new(&band(player), &band(vec![plain(1, 1)]), false);
    let mut rng = Rng::new(3);
    let mut c = Combat {
        p: &p,
        rng: &mut rng,
        sides: [
            SideState {
                units: p.start[0].clone(),
                next: 0,
                values: Default::default(),
                deaths: 0,
            },
            SideState {
                units: p.start[1].clone(),
                next: 0,
                values: Default::default(),
                deaths: 0,
            },
        ],
        next_uid: 100,
        forced_target: None,
        legs: Legs::of(&p),
    };
    c.sides[0].units[0].health = 0;
    c.process_deaths(0);
    assert_eq!(c.sides[0].units.len(), MAX_BOARD);
    assert_eq!(
        c.p.cards[c.sides[0].units[0].card as usize].id,
        "BG_ICC_026t"
    );
}

#[test]
fn titus_makes_deathrattles_trigger_again() {
    // Cord Puller (summons a 1/1 Microbot) next to Titus: two Microbots.
    let player = band(vec![
        Minion::new("BG29_611", 1, 1).tier(1),
        Minion::new("BG25_354", 0, 7).tier(5),
    ]);
    let p = Prepared::new(&player, &band(vec![]), false);
    let mut rng = Rng::new(1);
    let mut c = Combat {
        p: &p,
        rng: &mut rng,
        sides: [
            SideState {
                units: p.start[0].clone(),
                next: 0,
                values: Default::default(),
                deaths: 0,
            },
            SideState::default(),
        ],
        next_uid: 100,
        forced_target: None,
        legs: Legs::of(&p),
    };
    c.sides[0].units[0].health = 0;
    c.process_deaths(0);
    let ids: Vec<&str> = c.sides[0]
        .units
        .iter()
        .map(|u| c.p.cards[u.card as usize].id.as_str())
        .collect();
    assert_eq!(ids, ["BG_BOT_312t", "BG_BOT_312t", "BG25_354"]);
}

#[test]
fn rally_gains_attack_before_the_hit() {
    // Glim Guardian 1/4 (Rally: +2 Attack) against a 3/3: hits for 3 and
    // kills it on the first swing.
    let guardian = Minion::new("BG29_888", 1, 4).tier(1);
    let o = odds(vec![guardian, plain(0, 1)], vec![plain(3, 3)]);
    always(&o, CombatResult::Won);
}

#[test]
fn avenge_regains_divine_shield_and_with_it_taunt() {
    // Relentless Deflector 5/4 with Divine Shield counts as Taunt.
    let deflector = Minion::new("BG34_405", 5, 4).with(Keywords::DIVINE_SHIELD);
    let player = band(vec![plain(1, 1), plain(1, 1), plain(1, 1), deflector]);
    let p = Prepared::new(&player, &band(vec![]), false);
    let mut rng = Rng::new(5);
    let mut c = Combat {
        p: &p,
        rng: &mut rng,
        sides: [
            SideState {
                units: p.start[0].clone(),
                next: 0,
                values: Default::default(),
                deaths: 0,
            },
            SideState::default(),
        ],
        next_uid: 100,
        forced_target: None,
        legs: Legs::of(&p),
    };
    // Only the Deflector can be targeted while it has the shield.
    for _ in 0..20 {
        assert_eq!(c.choose_target(0), Some(4));
    }
    c.sides[0].units[3].kw = Keywords::NONE;
    for i in 0..3 {
        c.sides[0].units[i].health = 0;
    }
    c.process_deaths(0);
    let d = &c.sides[0].units[0];
    assert!(
        d.kw.contains(Keywords::DIVINE_SHIELD),
        "Avenge (3) after 3 deaths"
    );
}

#[test]
fn scarlet_survivor_gains_shield_once_at_six_attack() {
    let survivor = Minion::new("BG35_814", 3, 3);
    let wolf = Minion::new("BG36_207", 3, 6); // Rally: others +4/+1
    let player = band(vec![wolf, survivor]);
    let p = Prepared::new(&player, &band(vec![plain(0, 50)]), false);
    let mut rng = Rng::new(2);
    let mut c = Combat {
        p: &p,
        rng: &mut rng,
        sides: [
            SideState {
                units: p.start[0].clone(),
                next: 0,
                values: Default::default(),
                deaths: 0,
            },
            SideState {
                units: p.start[1].clone(),
                next: 0,
                values: Default::default(),
                deaths: 0,
            },
        ],
        next_uid: 100,
        forced_target: None,
        legs: Legs::of(&p),
    };
    c.swing(0, 1);
    let s = &c.sides[0].units[1];
    assert_eq!(s.attack, 7);
    assert!(s.kw.contains(Keywords::DIVINE_SHIELD));
}

#[test]
fn attack_order_goes_left_to_right_and_wraps() {
    let player = band(vec![plain(1, 50), plain(1, 50), plain(1, 50)]);
    let p = Prepared::new(&player, &band(vec![plain(0, 100)]), false);
    let mut rng = Rng::new(4);
    let mut c = Combat {
        p: &p,
        rng: &mut rng,
        sides: [
            SideState {
                units: p.start[0].clone(),
                next: 0,
                values: Default::default(),
                deaths: 0,
            },
            SideState {
                units: p.start[1].clone(),
                next: 0,
                values: Default::default(),
                deaths: 0,
            },
        ],
        next_uid: 100,
        forced_target: None,
        legs: Legs::of(&p),
    };
    let order: Vec<usize> = (0..5)
        .map(|_| {
            let ix = c.attacker_index(0).unwrap();
            c.attack_turn(0);
            ix
        })
        .collect();
    assert_eq!(order, [0, 1, 2, 0, 1]);
}

#[test]
fn a_summon_in_the_attackers_place_attacks_next() {
    // Bonehead (left) attacks and dies; its Skeletons take its place and the
    // pointer stays on that slot.
    let player = band(vec![Minion::new("BG28_300", 1, 1), plain(1, 50)]);
    let p = Prepared::new(&player, &band(vec![plain(5, 100)]), false);
    let mut rng = Rng::new(4);
    let mut c = Combat {
        p: &p,
        rng: &mut rng,
        sides: [
            SideState {
                units: p.start[0].clone(),
                next: 0,
                values: Default::default(),
                deaths: 0,
            },
            SideState {
                units: p.start[1].clone(),
                next: 0,
                values: Default::default(),
                deaths: 0,
            },
        ],
        next_uid: 100,
        forced_target: None,
        legs: Legs::of(&p),
    };
    c.attack_turn(0);
    let next = c.attacker_index(0).unwrap();
    assert_eq!(
        c.p.cards[c.sides[0].units[next].card as usize].id,
        "BG_ICC_026t"
    );
    assert_eq!(next, 0);
}

#[test]
fn same_seed_same_odds() {
    let a = odds(
        vec![plain(3, 4), plain(2, 2)],
        vec![plain(2, 3), plain(4, 1)],
    );
    let b = odds(
        vec![plain(3, 4), plain(2, 2)],
        vec![plain(2, 3), plain(4, 1)],
    );
    assert_eq!(a, b);
}

#[test]
fn unknown_cards_give_no_number_unless_asked() {
    let player = band(vec![Minion::new("NOT_A_CARD", 1, 1)]);
    let v = simulate(&player, &band(vec![]), &Options::default()).unwrap();
    match v {
        Verdict::NotModelled(c) => assert_eq!(c.not_modelled, ["NOT_A_CARD"]),
        Verdict::Odds(_) => panic!("must not give odds"),
    }
}

#[test]
fn start_of_combat_effects_count_only_when_played() {
    // Amber Guardian has a start-of-combat effect the engine does not play.
    let player = band(vec![Minion::new("BG24_500", 3, 2)]);
    let after = Options::default();
    assert!(matches!(
        simulate(&player, &band(vec![]), &after).unwrap(),
        Verdict::Odds(_)
    ));
    let before = Options {
        start_of_combat: true,
        ..after
    };
    assert!(matches!(
        simulate(&player, &band(vec![]), &before).unwrap(),
        Verdict::NotModelled(_)
    ));
}

#[test]
fn start_of_combat_buffs_apply_when_on() {
    // Hopebringer: Start of Combat: your minions +4/+3. A 1/1 and the
    // Hopebringer 4/3 against a 4/6: with the buff the 5/4 and 8/6 win.
    let hope = Minion::new("BG36_364", 4, 3).tier(2);
    let player = band(vec![hope, plain(1, 1)]);
    let opts = Options {
        start_of_combat: true,
        allow_unmodelled: true,
        trials: 500,
        ..Options::default()
    };
    let Verdict::Odds(o) = simulate(&player, &band(vec![plain(4, 6)]), &opts).unwrap() else {
        panic!()
    };
    always(&o, CombatResult::Won);
}

#[test]
fn leeroy_destroys_the_minion_that_killed_it_even_through_a_shield() {
    // A 6/2 Leeroy against a 2/9 with Divine Shield and a 0/1: Leeroy's hit
    // pops the shield, the 2/9 kills Leeroy, and the Deathrattle destroys it.
    let leeroy = Minion::new("BG23_318", 6, 2).tier(4);
    let shielded = plain(2, 9).with(Keywords::DIVINE_SHIELD);
    let p = Prepared::new(
        &band(vec![leeroy]),
        &band(vec![shielded, plain(0, 1)]),
        false,
    );
    let mut rng = Rng::new(8);
    let mut c = Combat {
        p: &p,
        rng: &mut rng,
        sides: [
            SideState {
                units: p.start[0].clone(),
                next: 0,
                values: Default::default(),
                deaths: 0,
            },
            SideState {
                units: p.start[1].clone(),
                next: 0,
                values: Default::default(),
                deaths: 0,
            },
        ],
        next_uid: 100,
        forced_target: None,
        legs: Legs::of(&p),
    };
    // The 2/9 attacks Leeroy (uids: Leeroy 1, 2/9 is 2).
    c.swing(1, 2);
    assert!(c.sides[0].units.is_empty());
    let left: Vec<i32> = c.sides[1].units.iter().map(|u| u.health).collect();
    assert_eq!(left, [1], "only the 0/1 is left");
}

#[test]
fn rally_strips_taunt_and_reborn_from_the_target() {
    // Sin'dorei Straight Shot 3/4 hits a 1/3 Taunt Reborn: the Rally removes
    // Reborn first, so after the swing no copy comes back.
    let shot = Minion::new("BG25_016", 3, 4).tier(3);
    let kws = Keywords::TAUNT.union(Keywords::REBORN);
    let mut target = plain(1, 3).with(kws);
    target.base = Some(BaseCard {
        attack: 1,
        health: 3,
        keywords: kws,
    });
    let p = Prepared::new(&band(vec![shot]), &band(vec![target.clone()]), false);
    let mut rng = Rng::new(8);
    let mut c = Combat {
        p: &p,
        rng: &mut rng,
        sides: [
            SideState {
                units: p.start[0].clone(),
                next: 0,
                values: Default::default(),
                deaths: 0,
            },
            SideState {
                units: p.start[1].clone(),
                next: 0,
                values: Default::default(),
                deaths: 0,
            },
        ],
        next_uid: 100,
        forced_target: None,
        legs: Legs::of(&p),
    };
    c.swing(0, 1);
    assert!(c.sides[1].units.is_empty(), "no Reborn copy");
    // A plain 3/4 leaves the Reborn copy behind.
    let p = Prepared::new(&band(vec![plain(3, 4)]), &band(vec![target]), false);
    let mut rng = Rng::new(8);
    let mut c = Combat {
        p: &p,
        rng: &mut rng,
        sides: [
            SideState {
                units: p.start[0].clone(),
                next: 0,
                values: Default::default(),
                deaths: 0,
            },
            SideState {
                units: p.start[1].clone(),
                next: 0,
                values: Default::default(),
                deaths: 0,
            },
        ],
        next_uid: 100,
        forced_target: None,
        legs: Legs::of(&p),
    };
    c.swing(0, 1);
    assert_eq!(c.sides[1].units.len(), 1);
}

#[test]
fn stealth_hides_taunt() {
    // Opponent: a Stealthed Taunt 0/1 and an open 0/1. The open one must be
    // the target every time.
    let opp = band(vec![
        plain(0, 1).with(Keywords::TAUNT.union(Keywords::STEALTH)),
        plain(0, 1),
    ]);
    let p = Prepared::new(&band(vec![plain(1, 1)]), &opp, false);
    let mut rng = Rng::new(6);
    let mut c = Combat {
        p: &p,
        rng: &mut rng,
        sides: [
            SideState {
                units: p.start[0].clone(),
                next: 0,
                values: Default::default(),
                deaths: 0,
            },
            SideState {
                units: p.start[1].clone(),
                next: 0,
                values: Default::default(),
                deaths: 0,
            },
        ],
        next_uid: 100,
        forced_target: None,
        legs: Legs::of(&p),
    };
    for _ in 0..20 {
        assert_eq!(c.choose_target(1), Some(3));
    }
}

#[test]
fn two_runs_merged_add_up() {
    let mut a = odds(vec![plain(1, 1), plain(1, 1)], vec![plain(1, 1)]);
    let b = a.clone();
    a.merge(&b);
    assert_eq!(a.trials, 2 * b.trials);
    assert_eq!(a.won, 2 * b.won);
    let dealt: u32 = a.damage_dealt.values().sum();
    assert_eq!(dealt, 2 * b.damage_dealt.values().sum::<u32>());
}
