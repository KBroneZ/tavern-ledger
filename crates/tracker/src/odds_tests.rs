//! Odds from made-up boards and card facts (no real card data, D-038).

use std::collections::BTreeMap;

use bg_parser::report::{CombatEntry, CombatValues, Round, Status};
use combat_sim::cards::{Trigger, EFFECTS};

use super::*;

const GAME: &str = "GT_BATTLEGROUNDS";
const DUOS: &str = "GT_BATTLEGROUNDS_DUO";

fn minion(card: &str, atk: i64, health: i64, position: i64) -> LogMinion {
    LogMinion {
        card_id: Some(card.into()),
        atk,
        health,
        position,
        golden: false,
        keywords: Vec::new(),
        sim_keywords: Vec::new(),
        gifts: Vec::new(),
        gift_unknown: false,
    }
}

fn entry(side: Side, seat: i64, board: Option<Vec<LogMinion>>) -> CombatEntry {
    CombatEntry {
        side,
        player_id: Some(seat),
        hero: None,
        board,
        values: None,
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
        own_shop_board: None,
        replay: None,
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
    let live = LiveFacts {
        own_now: Some(&own),
        own_values: None,
        next: Some(5),
        lobby: &[slot(2, 3), slot(5, 4)],
        damage_cap: Some(5),
        fights_first: None,
    };
    odds_input(&r, live).expect("odds")
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
        let live = LiveFacts {
            own_now: own,
            next,
            ..LiveFacts::default()
        };
        odds_input(r, live).expect_err("no odds").gap
    };
    assert_eq!(
        gap(&report(DUOS, vec![]), Some(&own), Some(5)),
        Gap::Order,
        "Duos without the game's order"
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
    let live = LiveFacts {
        own_now: Some(&own),
        next: Some(5),
        ..LiveFacts::default()
    };
    let i = odds_input(&r, live).expect("odds");
    assert!(i.seen_after_start_of_combat);
    assert!(run_odds(&i, &facts, Duration::from_secs(5)).start_of_combat);
}

/// Admiration: "Start of Combat: gain the Attack of the minion to the
/// left" (T-528).
const ADMIRATION: &str = "BG36_MidGameEffect_000t9";

#[test]
fn dark_gifts_reach_the_engine() {
    let gifted = LogMinion {
        gifts: vec![ADMIRATION, "BG36_MidGameEffect_000t13"],
        ..minion(PLAIN, 2, 3, 1)
    };
    let m = engine_minion(&gifted, &facts).expect("known card");
    assert_eq!(m.gifts, [ADMIRATION, "BG36_MidGameEffect_000t13"]);
    assert!(engine_minion(&minion(PLAIN, 2, 3, 1), &facts)
        .expect("known card")
        .gifts
        .is_empty());
}

#[test]
fn a_start_of_combat_gift_is_flagged() {
    let mut i = input();
    i.own.push(LogMinion {
        gifts: vec![ADMIRATION],
        ..minion(PLAIN, 1, 1, 2)
    });
    assert!(run_odds(&i, &facts, Duration::from_secs(5)).start_of_combat);

    // A gift that acted where the opponent's board was seen.
    let mut seen = round(3, 5, Some(vec![minion(PLAIN, 1, 1, 1)]));
    seen.entries[0].board = Some(vec![LogMinion {
        gifts: vec![ADMIRATION],
        ..minion(PLAIN, 1, 1, 1)
    }]);
    let r = report(GAME, vec![seen]);
    let own = [minion(PLAIN, 6, 6, 1)];
    let live = LiveFacts {
        own_now: Some(&own),
        next: Some(5),
        ..LiveFacts::default()
    };
    assert!(
        odds_input(&r, live)
            .expect("odds")
            .seen_after_start_of_combat
    );
}

#[test]
fn a_gift_the_log_does_not_name_makes_the_odds_approximate() {
    let mut i = input();
    i.opponent[0].gift_unknown = true;
    let o = run_odds(&i, &facts, Duration::from_secs(5));
    assert_eq!(o.gap, None);
    assert_eq!(o.approximate, [PLAIN]);
    assert!(run_odds(&input(), &facts, Duration::from_secs(5))
        .approximate
        .is_empty());
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

/// A Duos game: the player (seat 2) and their teammate (3) against seats 5
/// (the next opponent) and 6. The teammate fought on screen in round 3,
/// seat 6 in round 4, seat 5 in round 5.
fn duos_report(rounds: Vec<Round>) -> GameReport {
    let mut r = report(DUOS, rounds);
    r.teammate_player_id = Some(3);
    r.lobby = [(2, 1), (3, 1), (5, 2), (6, 2)]
        .into_iter()
        .map(|(seat, team)| bg_parser::report::LobbyPlayer {
            player_id: seat,
            hero: None,
            duo_team: Some(team),
            final_place: None,
            final_health: 0,
            triples: Vec::new(),
            trinkets: Vec::new(),
        })
        .collect();
    r
}

fn duos_round(number: i64, own: (i64, i64), foe: (i64, i64)) -> Round {
    Round {
        entries: vec![
            entry(Side::Own, own.0, Some(vec![minion(PLAIN, own.1, own.1, 1)])),
            entry(
                Side::Opponent,
                foe.0,
                Some(vec![minion(PLAIN, foe.1, foe.1, 1)]),
            ),
        ],
        ..round(number, foe.0, None)
    }
}

fn duos_rounds() -> Vec<Round> {
    vec![
        duos_round(3, (3, 7), (6, 1)),
        duos_round(4, (2, 2), (6, 2)),
        duos_round(5, (2, 2), (5, 3)),
    ]
}

fn duos_live(own: &[LogMinion], first: Option<bool>) -> LiveFacts<'_> {
    LiveFacts {
        own_now: Some(own),
        own_values: None,
        next: Some(5),
        lobby: &[],
        damage_cap: None,
        fights_first: first,
    }
}

#[test]
fn duos_odds_use_both_teams_last_boards_in_the_games_order() {
    let r = duos_report(duos_rounds());
    let own = [minion(PLAIN, 4, 4, 1)];
    let i = odds_input(&r, duos_live(&own, Some(false))).expect("odds");
    assert_eq!((i.opponent_seat, i.seen_round), (5, 5));
    let d = i.duos.clone().expect("Duos");
    assert!(!d.local_first);
    assert_eq!((d.teammate.seat, d.teammate.round), (3, 3));
    assert_eq!(d.teammate.board[0].atk, 7);
    // Seat 6's newest board, not the round 3 one.
    assert_eq!(
        (d.opponent_teammate.seat, d.opponent_teammate.round),
        (6, 4)
    );
    let mut missing = Vec::new();
    let (player, foe) = teams(&i, &facts, &mut missing);
    assert!(missing.is_empty());
    assert_eq!(
        player.first.minions[0].attack, 7,
        "the teammate fights first"
    );
    assert_eq!(player.second.expect("two boards").minions[0].attack, 4);
    assert_eq!(foe.first.minions[0].attack, 3, "the marked opponent first");
    assert_eq!(foe.second.expect("two boards").minions[0].attack, 2);

    let o = run_odds(&i, &facts, Duration::from_secs(5));
    assert_eq!(o.gap, None);
    assert_eq!(o.won, 1.0);
    assert_eq!(o.seen_round, Some(3), "the oldest board");
    let rounds: Vec<_> = o.boards.iter().map(|b| (b.role, b.seat, b.round)).collect();
    assert_eq!(
        rounds,
        [
            (Role::Teammate, 3, Some(3)),
            (Role::Opponent, 5, Some(5)),
            (Role::OpponentTeammate, 6, Some(4))
        ]
    );
}

#[test]
fn duos_names_the_boards_never_seen() {
    // Only seat 5 has been met; the teammate never fought on screen.
    let r = duos_report(vec![duos_round(5, (2, 2), (5, 3))]);
    let own = [minion(PLAIN, 4, 4, 1)];
    let no = odds_input(&r, duos_live(&own, Some(true))).expect_err("no odds");
    assert_eq!((no.gap, no.seat), (Gap::MissingBoard, Some(5)));
    let rounds: Vec<_> = no.boards.iter().map(|b| (b.role, b.round)).collect();
    assert_eq!(
        rounds,
        [
            (Role::Teammate, None),
            (Role::Opponent, Some(5)),
            (Role::OpponentTeammate, None)
        ]
    );
    // The overlay gets the same list.
    assert_eq!(no.odds().boards, no.boards);
}

#[test]
fn duos_without_the_games_order_or_teams_gives_no_number() {
    let own = [minion(PLAIN, 4, 4, 1)];
    let r = duos_report(duos_rounds());
    let order = |r: &GameReport, first| odds_input(r, duos_live(&own, first)).unwrap_err().gap;
    assert_eq!(order(&r, None), Gap::Order);
    let mut no_teams = duos_report(duos_rounds());
    no_teams.lobby.clear();
    assert_eq!(order(&no_teams, Some(true)), Gap::Order);
    let mut no_mate = duos_report(duos_rounds());
    no_mate.teammate_player_id = None;
    assert_eq!(order(&no_mate, Some(true)), Gap::Order);
}

#[test]
fn duos_odds_flag_a_start_of_combat_card_where_a_board_was_seen() {
    let soc = EFFECTS
        .iter()
        .find(|(_, r)| r.triggers.contains(&Trigger::StartOfCombatUnmodelled))
        .map(|(id, _)| *id)
        .expect("a start of combat card");
    let mut rounds = duos_rounds();
    // The teammate's round had the card on the other side.
    rounds[0].entries[1].board = Some(vec![minion(soc, 1, 1, 1)]);
    let r = duos_report(rounds);
    let own = [minion(PLAIN, 4, 4, 1)];
    let i = odds_input(&r, duos_live(&own, Some(true))).expect("odds");
    assert!(i.seen_after_start_of_combat);
}

fn values(undead_attack: i64, blood_gem: (i64, i64)) -> Option<CombatValues> {
    Some(CombatValues {
        undead_attack,
        blood_gem,
        ..CombatValues::default()
    })
}

#[test]
fn each_board_plays_with_its_game_long_values() {
    // T-610: the player's values now, the opponent's from the combat their
    // board was seen in.
    let mut rounds = vec![round(3, 5, Some(vec![minion(PLAIN, 1, 1, 1)]))];
    rounds[0].entries[1].values = values(4, (0, 0));
    rounds[0].entries[0].values = values(9, (9, 9));
    let r = report(GAME, rounds);
    let own = [minion(PLAIN, 6, 6, 1)];
    let live = LiveFacts {
        own_now: Some(&own),
        own_values: values(0, (2, 1)),
        next: Some(5),
        ..LiveFacts::default()
    };
    let i = odds_input(&r, live).expect("odds");
    assert_eq!(i.own_values.blood_gem, (2, 1), "now, not the combat's");
    assert_eq!(i.own_values.undead_attack, 0);
    assert_eq!(i.opponent_values.undead_attack, 4);
    let mut missing = Vec::new();
    let (player, foe) = teams(&i, &facts, &mut missing);
    assert_eq!(player.first.values.blood_gem, (2, 1));
    assert_eq!(foe.first.values.undead_attack, 4);

    // Without values (none read yet): the printed numbers, as before.
    let none = odds_input(
        &r,
        LiveFacts {
            own_values: None,
            ..live
        },
    )
    .expect("odds");
    let (player, _) = teams(&none, &facts, &mut missing);
    assert_eq!(player.first.values, combat_sim::GameValues::default());
}

#[test]
fn game_long_values_past_what_the_report_saves_play_as_unknown() {
    let mut rounds = vec![round(3, 5, Some(vec![minion(PLAIN, 1, 1, 1)]))];
    rounds[0].entries[1].values = values(i64::MAX, (-2, 3));
    let r = report(GAME, rounds);
    let own = [minion(PLAIN, 6, 6, 1)];
    let live = LiveFacts {
        own_now: Some(&own),
        next: Some(5),
        ..LiveFacts::default()
    };
    let i = odds_input(&r, live).expect("odds");
    let mut missing = Vec::new();
    let (_, foe) = teams(&i, &facts, &mut missing);
    assert_eq!(foe.first.values.undead_attack, 0);
    assert_eq!(foe.first.values.blood_gem, (0, 3));
}

#[test]
fn duos_boards_play_with_their_own_game_long_values() {
    let mut rounds = duos_rounds();
    // Round 3: the teammate (seat 3) against seat 6; round 4: seat 6 again.
    rounds[0].entries[0].values = values(3, (0, 0));
    rounds[1].entries[1].values = values(5, (0, 0));
    rounds[2].entries[1].values = values(0, (4, 4));
    let r = duos_report(rounds);
    let own = [minion(PLAIN, 4, 4, 1)];
    let live = LiveFacts {
        own_values: values(0, (1, 1)),
        ..duos_live(&own, Some(true))
    };
    let i = odds_input(&r, live).expect("odds");
    let d = i.duos.clone().expect("Duos");
    assert_eq!(d.teammate.values.undead_attack, 3);
    assert_eq!(d.opponent_teammate.values.undead_attack, 5);
    let mut missing = Vec::new();
    let (player, foe) = teams(&i, &facts, &mut missing);
    assert_eq!(player.first.values.blood_gem, (1, 1), "the player first");
    assert_eq!(player.second.expect("two").values.undead_attack, 3);
    assert_eq!(foe.first.values.blood_gem, (4, 4), "seat 5, round 5");
    assert_eq!(foe.second.expect("two").values.undead_attack, 5);
}
