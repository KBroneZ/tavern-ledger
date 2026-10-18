//! Odds from made-up boards and card facts (no real card data, D-038).

use std::collections::BTreeMap;

use bg_parser::report::{CombatEntry, CombatValues, LobbyPlayer, Round, Status};
use combat_sim::cards::{Trigger, EFFECTS};

use super::*;

const GAME: &str = "GT_BATTLEGROUNDS";
const DUOS: &str = "GT_BATTLEGROUNDS_DUO";

fn minion(card: &str, atk: i64, health: i64, position: i64) -> LogMinion {
    LogMinion {
        counters: [0, 0],
        card_id: Some(card.into()),
        atk,
        health,
        position,
        golden: false,
        keywords: Vec::new(),
        sim_keywords: Vec::new(),
        gifts: Vec::new(),
        gift_unknown: false,
        buffs: Vec::new(),
    }
}

fn entry(side: Side, seat: i64, board: Option<Vec<LogMinion>>) -> CombatEntry {
    CombatEntry {
        side,
        player_id: Some(seat),
        hero: None,
        board,
        values: None,
        before: None,
        kit: None,
    }
}

fn round_of(number: i64, entries: Vec<CombatEntry>) -> Round {
    Round {
        number,
        entries,
        own_health_after: None,
        health_after: BTreeMap::new(),
        result: None,
        attacks: Vec::new(),
        own_shop_board: None,
        hidden_triggers: Vec::new(),
        replay: None,
    }
}

/// A Solo combat: the player (seat 2) against `opponent`.
fn round(
    number: i64,
    own: Option<Vec<LogMinion>>,
    opponent: i64,
    board: Option<Vec<LogMinion>>,
) -> Round {
    round_of(
        number,
        vec![
            entry(Side::Own, 2, own),
            entry(Side::Opponent, opponent, board),
        ],
    )
}

fn report(game_type: &str, rounds: Vec<Round>) -> GameReport {
    let mut r = GameReport::new(1, Status::Incomplete, Some(game_type.into()), Some(1));
    r.local_player_id = Some(2);
    r.rounds = rounds;
    r
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

fn plain(atk: i64) -> Vec<LogMinion> {
    vec![minion(PLAIN, atk, atk, 1)]
}

/// The lobby the game shows.
const LOBBY: [LobbySlot; 2] = [
    LobbySlot {
        player_id: 2,
        place: None,
        tech_level: Some(3),
        health: None,
        armor: None,
    },
    LobbySlot {
        player_id: 5,
        place: None,
        tech_level: Some(4),
        health: None,
        armor: None,
    },
];

/// The game is playing combat `round` now.
fn live(round: i64) -> LiveFacts<'static> {
    LiveFacts {
        combat_round: Some(round),
        lobby: &LOBBY,
        damage_cap: Some(5),
    }
}

/// Combat 4 is on: the player's 6/6 against seat 5's 1/1. Combat 3 (a 9/9
/// board of seat 5's) is over and must never be used.
fn solo_report() -> GameReport {
    report(
        GAME,
        vec![
            round(3, Some(plain(1)), 5, Some(plain(9))),
            round(4, Some(plain(6)), 5, Some(plain(1))),
        ],
    )
}

fn input() -> OddsInput {
    odds_input(&solo_report(), live(4)).expect("odds")
}

#[test]
fn the_input_is_the_two_boards_of_the_combat_on_screen() {
    let i = input();
    assert_eq!((i.opponent_seat, i.round), (5, 4));
    assert_eq!((i.own[0].atk, i.opponent[0].atk), (6, 1));
    assert_eq!(
        (i.own_tier, i.opponent_tier, i.damage_cap),
        (Some(3), Some(4), Some(5))
    );
}

#[test]
fn there_are_no_odds_outside_a_combat() {
    // A shop turn: the game plays no combat, however many it has played.
    let live = LiveFacts {
        combat_round: None,
        ..live(4)
    };
    let no = odds_input(&solo_report(), live).expect_err("recruit phase");
    assert_eq!((no.gap, no.seat), (Gap::NotInCombat, None));
}

#[test]
fn a_combat_never_gets_the_odds_of_an_earlier_one() {
    // Combat 5 has begun but its boards are not in the report yet; combat 4's
    // are, and must not stand in for them.
    let no = odds_input(&solo_report(), live(5)).expect_err("boards not in");
    assert_eq!(no.gap, Gap::NoBoardSeen);

    // Once they are in, those are the ones played.
    let mut r = solo_report();
    r.rounds.push(round(5, Some(plain(2)), 5, Some(plain(3))));
    let i = odds_input(&r, live(5)).expect("odds");
    assert_eq!((i.round, i.own[0].atk, i.opponent[0].atk), (5, 2, 3));
    let i = odds_input(&r, live(4)).expect("odds");
    assert_eq!((i.round, i.own[0].atk, i.opponent[0].atk), (4, 6, 1));
}

#[test]
fn a_board_the_log_does_not_have_gives_no_number() {
    let missing = |own, board| {
        let r = report(GAME, vec![round(4, own, 5, board)]);
        odds_input(&r, live(4)).expect_err("a board is missing")
    };
    assert_eq!(missing(Some(plain(1)), None).gap, Gap::NoBoardSeen);
    assert_eq!(missing(None, Some(plain(1))).gap, Gap::NoBoardSeen);
    // The seat is known when the opponent's board is in.
    assert_eq!(missing(None, Some(plain(1))).seat, Some(5));
}

#[test]
fn a_sure_win_gives_the_damage_after_the_cap() {
    let o = run_odds(&input(), &facts, Duration::from_secs(5));
    assert_eq!(o.gap, None);
    assert_eq!((o.won, o.trials, o.round), (1.0, TRIALS, Some(4)));
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

fn start_of_combat_card() -> &'static str {
    EFFECTS
        .iter()
        .find(|(_, r)| r.triggers.contains(&Trigger::StartOfCombatUnmodelled))
        .map(|(id, _)| *id)
        .expect("a start of combat card")
}

#[test]
fn a_start_of_combat_card_is_flagged() {
    let soc = start_of_combat_card();
    let mut i = input();
    i.own.push(minion(soc, 1, 1, 2));
    let o = run_odds(&i, &facts, Duration::from_secs(5));
    assert!(o.start_of_combat);
    assert!(!run_odds(&input(), &facts, Duration::from_secs(5)).start_of_combat);
    assert!(!input().seen_after_start_of_combat);

    // The card acted when the boards entered play and is gone from the
    // board: the boards may carry its effects.
    let mut r = solo_report();
    r.rounds[1].entries[0].board = Some(vec![minion(soc, 1, 1, 1)]);
    let i = odds_input(&r, live(4)).expect("odds");
    assert!(i.seen_after_start_of_combat);

    // Only the combat on screen counts, not an earlier one.
    let mut r = solo_report();
    r.rounds[0].entries[0].board = Some(vec![minion(soc, 1, 1, 1)]);
    assert!(
        !odds_input(&r, live(4))
            .expect("odds")
            .seen_after_start_of_combat
    );
}

#[test]
fn a_build_the_rules_were_not_checked_against_gets_odds_and_a_warning() {
    let run = |build: Option<i64>| {
        let mut i = input();
        i.build = build;
        run_odds(&i, &facts, Duration::from_secs(5))
    };
    let new = run(Some(999_999));
    assert_eq!(new.new_build, Some(999_999));
    // The odds still come back.
    assert_eq!(new.gap, None);
    assert!(new.trials > 0);
    assert!((new.won + new.tied + new.lost - 1.0).abs() < 1e-9);

    for checked in [253_216, 254_477] {
        assert_eq!(run(Some(checked)).new_build, None, "build {checked}");
    }
    // A build the log did not give is not one to warn about.
    assert_eq!(run(None).new_build, None);
}

#[test]
fn every_build_the_rules_were_checked_against_is_one_the_parser_was_tested_on() {
    for build in combat_sim::cards::CHECKED_BUILDS {
        assert!(
            bg_parser::TESTED_BUILDS.contains(build),
            "build {build} is checked by the engine but not tested by the parser"
        );
    }
}

#[test]
fn the_input_carries_the_reports_build() {
    let mut r = solo_report();
    r.build = Some(254_477);
    assert_eq!(odds_input(&r, live(4)).expect("odds").build, Some(254_477));
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

    // A gift that acted where the boards entered play.
    let mut r = solo_report();
    r.rounds[1].entries[1].board = Some(vec![LogMinion {
        gifts: vec![ADMIRATION],
        ..minion(PLAIN, 1, 1, 1)
    }]);
    assert!(
        odds_input(&r, live(4))
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
/// and 6. In combat 5 seat 5 fights first on their team.
fn duos_report(rounds: Vec<Round>) -> GameReport {
    let mut r = report(DUOS, rounds);
    r.teammate_player_id = Some(3);
    r.lobby = [(2, 1), (3, 1), (5, 2), (6, 2)]
        .into_iter()
        .map(|(seat, team)| LobbyPlayer {
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

/// A Duos combat's entries in the order the boards entered play: (side,
/// seat, attack of its one minion).
fn duos_round(number: i64, entered: &[(Side, i64, i64)]) -> Round {
    round_of(
        number,
        entered
            .iter()
            .map(|(side, seat, atk)| entry(*side, *seat, Some(plain(*atk))))
            .collect(),
    )
}

/// Combat 5 with all four boards in: the teammate first, then the player;
/// seat 5 first, then seat 6.
fn duos_combat_5() -> Round {
    duos_round(
        5,
        &[
            (Side::Own, 3, 7),
            (Side::Opponent, 5, 3),
            (Side::Own, 2, 4),
            (Side::Opponent, 6, 2),
        ],
    )
}

#[test]
fn duos_odds_use_the_four_boards_of_the_combat_in_the_order_they_entered() {
    // Combat 4 had other boards: never used.
    let earlier = duos_round(
        4,
        &[
            (Side::Own, 2, 9),
            (Side::Opponent, 6, 9),
            (Side::Own, 3, 9),
            (Side::Opponent, 5, 9),
        ],
    );
    let r = duos_report(vec![earlier, duos_combat_5()]);
    let i = odds_input(&r, live(5)).expect("odds");
    assert_eq!((i.opponent_seat, i.round), (5, 5));
    let d = i.duos.clone().expect("Duos");
    assert!(!d.local_first, "the teammate's board came in first");
    assert_eq!((d.teammate.seat, d.teammate.round), (3, 5));
    assert_eq!(d.teammate.board[0].atk, 7);
    assert_eq!(
        (d.opponent_teammate.seat, d.opponent_teammate.round),
        (6, 5)
    );
    let mut missing = Vec::new();
    let (player, foe) = teams(&i, &facts, &mut missing);
    assert!(missing.is_empty());
    assert_eq!(
        player.first.minions[0].attack, 7,
        "the teammate fights first"
    );
    assert_eq!(player.second.expect("two boards").minions[0].attack, 4);
    assert_eq!(foe.first.minions[0].attack, 3, "seat 5 first");
    assert_eq!(foe.second.expect("two boards").minions[0].attack, 2);

    let o = run_odds(&i, &facts, Duration::from_secs(5));
    assert_eq!(o.gap, None);
    assert_eq!(o.won, 1.0);
    assert_eq!(o.round, Some(5));
    let boards: Vec<_> = o.boards.iter().map(|b| (b.role, b.seat, b.round)).collect();
    assert_eq!(
        boards,
        [
            (Role::Teammate, 3, Some(5)),
            (Role::Opponent, 5, Some(5)),
            (Role::OpponentTeammate, 6, Some(5))
        ]
    );
}

#[test]
fn duos_with_the_player_first_puts_their_board_first() {
    let mut r = duos_report(vec![duos_combat_5()]);
    r.rounds[0].entries.swap(0, 2);
    let i = odds_input(&r, live(5)).expect("odds");
    assert!(i.duos.expect("Duos").local_first);
}

#[test]
fn duos_names_the_boards_not_in_play_yet() {
    // Only the first board of each team has entered.
    let r = duos_report(vec![duos_round(
        5,
        &[(Side::Own, 3, 7), (Side::Opponent, 5, 3)],
    )]);
    let no = odds_input(&r, live(5)).expect_err("no odds yet");
    assert_eq!((no.gap, no.seat), (Gap::MissingBoard, Some(5)));
    let boards: Vec<_> = no.boards.iter().map(|b| (b.role, b.round)).collect();
    assert_eq!(
        boards,
        [
            (Role::Own, None),
            (Role::Teammate, Some(5)),
            (Role::Opponent, Some(5)),
            (Role::OpponentTeammate, None)
        ]
    );
    // The overlay gets the same list.
    assert_eq!(no.odds().boards, no.boards);
}

#[test]
fn duos_without_a_board_of_the_combat_or_the_teams_gives_no_number() {
    let gap = |r: &GameReport| odds_input(r, live(5)).unwrap_err().gap;
    // Combat 5's boards are not in yet; combat 4's are.
    let r = duos_report(vec![duos_round(
        4,
        &[(Side::Own, 3, 7), (Side::Opponent, 5, 3)],
    )]);
    assert_eq!(gap(&r), Gap::NoBoardSeen);
    let mut no_teams = duos_report(vec![duos_combat_5()]);
    no_teams.lobby.clear();
    assert_eq!(gap(&no_teams), Gap::BadBoard);
    let mut no_mate = duos_report(vec![duos_combat_5()]);
    no_mate.teammate_player_id = None;
    assert_eq!(gap(&no_mate), Gap::BadBoard);
}

#[test]
fn duos_odds_flag_a_start_of_combat_card_on_a_board_of_the_combat() {
    let mut r = duos_report(vec![duos_combat_5()]);
    r.rounds[0].entries[3].board = Some(vec![minion(start_of_combat_card(), 1, 1, 1)]);
    let i = odds_input(&r, live(5)).expect("odds");
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
fn each_board_plays_with_the_game_long_values_it_entered_with() {
    // T-610: both boards' values are the ones of the combat on screen, not
    // the player's now nor an earlier combat's.
    let mut r = solo_report();
    r.rounds[0].entries[0].values = values(9, (9, 9));
    r.rounds[1].entries[0].values = values(0, (2, 1));
    r.rounds[1].entries[1].values = values(4, (0, 0));
    let i = odds_input(&r, live(4)).expect("odds");
    assert_eq!(i.own_values.blood_gem, (2, 1));
    assert_eq!(i.own_values.undead_attack, 0);
    assert_eq!(i.opponent_values.undead_attack, 4);
    let mut missing = Vec::new();
    let (player, foe) = teams(&i, &facts, &mut missing);
    assert_eq!(player.first.values.blood_gem, (2, 1));
    assert_eq!(foe.first.values.undead_attack, 4);

    // Without values (none read): the printed numbers.
    let none = odds_input(&solo_report(), live(4)).expect("odds");
    let (player, _) = teams(&none, &facts, &mut missing);
    assert_eq!(player.first.values, combat_sim::GameValues::default());
}

#[test]
fn game_long_values_past_what_the_report_saves_play_as_unknown() {
    let mut r = solo_report();
    r.rounds[1].entries[1].values = values(i64::MAX, (-2, 3));
    let i = odds_input(&r, live(4)).expect("odds");
    let mut missing = Vec::new();
    let (_, foe) = teams(&i, &facts, &mut missing);
    assert_eq!(foe.first.values.undead_attack, 0);
    assert_eq!(foe.first.values.blood_gem, (0, 3));
}

#[test]
fn duos_boards_play_with_their_own_game_long_values() {
    let mut r = duos_report(vec![duos_combat_5()]);
    // Entry order: teammate, seat 5, the player, seat 6.
    r.rounds[0].entries[0].values = values(3, (0, 0));
    r.rounds[0].entries[1].values = values(0, (4, 4));
    r.rounds[0].entries[2].values = values(0, (1, 1));
    r.rounds[0].entries[3].values = values(5, (0, 0));
    let i = odds_input(&r, live(5)).expect("odds");
    let d = i.duos.clone().expect("Duos");
    assert_eq!(d.teammate.values.undead_attack, 3);
    assert_eq!(d.opponent_teammate.values.undead_attack, 5);
    let mut missing = Vec::new();
    let (player, foe) = teams(&i, &facts, &mut missing);
    assert_eq!(player.first.values.undead_attack, 3, "the teammate first");
    assert_eq!(player.second.expect("two").values.blood_gem, (1, 1));
    assert_eq!(foe.first.values.blood_gem, (4, 4), "seat 5");
    assert_eq!(foe.second.expect("two").values.undead_attack, 5);
}

#[path = "odds_estimate_tests.rs"]
mod estimate;
