//! Odds from made-up boards and card facts (no real card data, D-038).

use std::collections::BTreeMap;

use bg_parser::report::{CombatEntry, Round, Status};
use combat_sim::cards::{Trigger, EFFECTS};

use super::*;

const GAME: &str = "GT_BATTLEGROUNDS";

fn minion(card: &str, atk: i64, health: i64, position: i64) -> LogMinion {
    LogMinion {
        card_id: Some(card.into()),
        atk,
        health,
        position,
        golden: false,
        keywords: Vec::new(),
        sim_keywords: Vec::new(),
    }
}

fn entry(side: Side, seat: i64, board: Option<Vec<LogMinion>>) -> CombatEntry {
    CombatEntry {
        side,
        player_id: Some(seat),
        hero: None,
        board,
    }
}

fn round(number: i64, opponent: i64, board: Option<Vec<LogMinion>>) -> Round {
    Round {
        number,
        entries: vec![
            entry(Side::Own, 2, Some(vec![])),
            entry(Side::Opponent, opponent, board),
        ],
        own_health_after: None,
        health_after: BTreeMap::new(),
        result: None,
        attacks: Vec::new(),
    }
}

fn report(game_type: &str, rounds: Vec<Round>) -> GameReport {
    let mut r = GameReport::new(1, Status::Incomplete, Some(game_type.into()), Some(1));
    r.local_player_id = Some(2);
    r.rounds = rounds;
    r
}

fn slot(player_id: i64, tier: i64) -> LobbySlot {
    LobbySlot {
        player_id,
        place: None,
        tech_level: Some(tier),
    }
}

/// Every card is a tier 1 minion with no type, its printed stats 1/1.
fn facts(_: &str) -> Option<Facts> {
    Some(Facts {
        tier: Some(1),
        tribes: Tribes::NONE,
        base: BaseCard {
            attack: 1,
            health: 1,
            keywords: Keywords::NONE,
        },
    })
}

/// A plain card with no combat effect (round 1's list).
const PLAIN: &str = "BGS_119";

fn input() -> OddsInput {
    let r = report(
        GAME,
        vec![
            round(3, 5, Some(vec![minion(PLAIN, 1, 1, 1)])),
            round(4, 6, None),
            // The combat with seat 6 never played out: unknown, skipped.
            round(5, 6, Some(vec![minion(PLAIN, 9, 9, 1)])),
            round(6, 5, None),
        ],
    );
    let own = [minion(PLAIN, 6, 6, 1)];
    odds_input(&r, Some(&own), Some(5), &[slot(2, 3), slot(5, 4)], Some(5)).expect("odds")
}

#[test]
fn the_input_is_the_next_opponents_last_board_seen() {
    let i = input();
    assert_eq!((i.opponent_seat, i.seen_round), (5, 3));
    assert_eq!(i.opponent[0].atk, 1);
    assert_eq!(
        (i.own_tier, i.opponent_tier, i.damage_cap),
        (Some(3), Some(4), Some(5))
    );
}

#[test]
fn a_sure_win_gives_the_damage_after_the_cap() {
    let o = run_odds(&input(), &facts, Duration::from_secs(5));
    assert_eq!(o.gap, None);
    assert_eq!((o.won, o.trials), (1.0, TRIALS));
    // Tier 3 + one tier 1 minion = 4, under the cap of 5.
    let dealt = o.dealt.expect("tiers known");
    assert_eq!((dealt.low, dealt.likely, dealt.high), (4, 4, 4));
    assert_eq!(o.taken, None, "never lost");
    let mut capped = input();
    capped.damage_cap = Some(2);
    assert_eq!(
        run_odds(&capped, &facts, Duration::from_secs(5))
            .dealt
            .unwrap()
            .likely,
        2
    );
}

#[test]
fn a_tight_budget_still_runs_one_batch() {
    let o = run_odds(&input(), &facts, Duration::ZERO);
    assert_eq!(o.trials, BATCH);
    assert_eq!(o.gap, None);
}

#[test]
fn unknown_tiers_give_no_damage_not_zero() {
    let no_tier = |_: &str| facts("").map(|f| Facts { tier: None, ..f });
    let o = run_odds(&input(), &no_tier, Duration::from_secs(5));
    assert_eq!(o.won, 1.0);
    assert_eq!(o.dealt, None);
}

#[test]
fn each_reason_for_no_odds_is_named() {
    let r = report(GAME, vec![round(1, 5, Some(vec![minion(PLAIN, 1, 1, 1)]))]);
    let own = [minion(PLAIN, 2, 2, 1)];
    let gap = |r: &GameReport, own: Option<&[LogMinion]>, next| {
        odds_input(r, own, next, &[], None)
            .expect_err("no odds")
            .gap
    };
    assert_eq!(
        gap(&report("GT_BATTLEGROUNDS_DUO", vec![]), Some(&own), Some(5)),
        Gap::Duos
    );
    assert_eq!(gap(&r, Some(&own), None), Gap::NoNextOpponent);
    assert_eq!(gap(&r, None, Some(5)), Gap::NotInShop);
    assert_eq!(gap(&r, Some(&own), Some(7)), Gap::NoBoardSeen);

    let i = input();
    let none = |_: &str| None;
    let o = run_odds(&i, &none, Duration::from_secs(1));
    assert_eq!(o.gap, Some(Gap::NoCardData));
    assert_eq!(o.missing, [PLAIN]);
    assert_eq!((o.won, o.trials), (0.0, 0));

    let mut odd = input();
    odd.opponent = vec![minion("SYN_NOT_A_CARD", 1, 1, 1)];
    let o = run_odds(&odd, &facts, Duration::from_secs(1));
    assert_eq!(o.gap, Some(Gap::NotModelled));
    assert_eq!(o.missing, ["SYN_NOT_A_CARD"]);
}

#[test]
fn a_start_of_combat_card_is_flagged() {
    let soc = EFFECTS
        .iter()
        .find(|(_, r)| r.triggers.contains(&Trigger::StartOfCombatUnmodelled))
        .map(|(id, _)| *id)
        .expect("a start of combat card");
    let mut i = input();
    i.own.push(minion(soc, 1, 1, 2));
    let o = run_odds(&i, &facts, Duration::from_secs(5));
    assert!(o.start_of_combat);
    assert!(!run_odds(&input(), &facts, Duration::from_secs(5)).start_of_combat);
    assert!(!input().seen_after_start_of_combat);

    // The player's card acted when the board was seen, and is gone now:
    // the board seen may carry its effects.
    let mut seen = round(3, 5, Some(vec![minion(PLAIN, 1, 1, 1)]));
    seen.entries[0].board = Some(vec![minion(soc, 1, 1, 1)]);
    let r = report(GAME, vec![seen]);
    let own = [minion(PLAIN, 6, 6, 1)];
    let i = odds_input(&r, Some(&own), Some(5), &[], None).expect("odds");
    assert!(i.seen_after_start_of_combat);
    assert!(run_odds(&i, &facts, Duration::from_secs(5)).start_of_combat);
}

#[test]
fn card_data_words_become_facts() {
    let f = Facts::from_names(
        2,
        3,
        Some(4),
        &["MECHANICAL".into(), "ODD".into()],
        &["DIVINE_SHIELD".into()],
    );
    assert!(f.tribes.is(Tribes::MECH));
    assert!(f.base.keywords.contains(Keywords::DIVINE_SHIELD));
    assert_eq!((f.base.attack, f.base.health, f.tier), (2, 3, Some(4)));
}

#[test]
fn the_spread_takes_percentiles() {
    let counts: BTreeMap<i32, u32> = [(1, 10), (5, 80), (9, 10)].into_iter().collect();
    assert_eq!(
        spread(&counts, None),
        Some(Spread {
            low: 1,
            likely: 5,
            high: 5
        })
    );
    let wide: BTreeMap<i32, u32> = [(1, 5), (5, 80), (9, 15)].into_iter().collect();
    assert_eq!(
        spread(&wide, Some(8)),
        Some(Spread {
            low: 5,
            likely: 5,
            high: 8
        })
    );
    assert_eq!(spread(&BTreeMap::new(), None), None);
}
