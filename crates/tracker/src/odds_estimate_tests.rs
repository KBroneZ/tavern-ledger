//! The early estimate from the boards before start of combat and the heroes'
//! powers and trinkets (T-628, D-165), from made-up boards (D-038).

use bg_parser::report::OpeningLeg;

use super::*;

/// Fire Invocation: at the start of combat the left-most minion's Attack
/// doubles.
const FIRE: &str = "BG22_HERO_001p_t2";

/// A hero with the one power `power`, no numbers.
fn kit(power: &str) -> HeroKit {
    HeroKit {
        powers: vec![LogKitCard {
            card_id: power.into(),
            ..Default::default()
        }],
        trinkets: Vec::new(),
    }
}

/// A leg of combat `round` between its start of combat and first attack.
fn opening(round: i64, side: Side, seat: i64, board: Vec<LogMinion>, kit: HeroKit) -> OpeningLeg {
    OpeningLeg {
        round,
        side,
        player_id: Some(seat),
        hero: None,
        board,
        kit,
        values: CombatValues::default(),
    }
}

/// Combat 4 has begun: the player's 2/3 with Fire Invocation against seat
/// 5's 3/3; no board has reached its first attack. Combat 3 is over.
fn begun_report() -> GameReport {
    let mut r = report(GAME, vec![round(3, Some(plain(1)), 5, Some(plain(9)))]);
    r.opening = vec![
        opening(4, Side::Own, 2, vec![minion(PLAIN, 2, 3, 1)], kit(FIRE)),
        opening(
            4,
            Side::Opponent,
            5,
            vec![minion(PLAIN, 3, 3, 1)],
            HeroKit::default(),
        ),
    ];
    r
}

#[test]
fn the_estimate_plays_start_of_combat_on_the_boards_before_it() {
    let i = odds_input(&begun_report(), live(4)).expect("an estimate");
    assert!(i.estimate);
    assert_eq!((i.opponent_seat, i.round), (5, 4));
    assert_eq!(i.own[0].atk, 2);
    assert_eq!(i.own_kit, Some(kit(FIRE)));
    assert!(!i.seen_after_start_of_combat);
    let o = run_odds(&i, &facts, Duration::from_secs(5));
    assert_eq!(o.gap, None);
    assert!(o.estimate);
    assert!(!o.start_of_combat, "start of combat is played, not missed");
    assert_eq!(o.tied, 1.0, "the doubled 4 Attack trades with the 3/3");
}

#[test]
fn the_first_attacks_boards_replace_the_estimate_and_keep_the_kits() {
    let mut r = begun_report();
    r.opening.clear();
    // Made up: the same boards at the first attack, which are played without
    // start of combat (its effects are on them already).
    let mut fought = round(
        4,
        Some(vec![minion(PLAIN, 2, 3, 1)]),
        5,
        Some(vec![minion(PLAIN, 3, 3, 1)]),
    );
    fought.entries[0].kit = Some(kit(FIRE));
    r.rounds.push(fought);
    let i = odds_input(&r, live(4)).expect("odds");
    assert!(!i.estimate);
    assert_eq!(i.own_kit, Some(kit(FIRE)));
    let o = run_odds(&i, &facts, Duration::from_secs(5));
    assert!(!o.estimate);
    assert_eq!(o.lost, 1.0, "Fire Invocation is not played again");

    // Once in, they win over a leg still listed as begun.
    r.opening = begun_report().opening;
    assert!(!odds_input(&r, live(4)).expect("odds").estimate);
}

#[test]
fn an_estimate_with_a_start_of_combat_source_not_played_gives_no_number_yet() {
    let mut r = begun_report();
    r.opening[0].board = vec![minion(start_of_combat_card(), 1, 1, 1)];
    let i = odds_input(&r, live(4)).expect("an estimate's input");
    let o = run_odds(&i, &facts, Duration::from_secs(5));
    assert_eq!(
        (o.gap, o.seat, o.round),
        (Some(Gap::NoBoardSeen), Some(5), Some(4))
    );
    assert!(o.missing.is_empty() && !o.estimate);
}

#[test]
fn an_estimate_needs_both_legs_of_the_combat_on_screen() {
    let mut r = begun_report();
    r.opening.remove(1);
    let no = odds_input(&r, live(4)).expect_err("the opponent's leg has not begun");
    assert_eq!(no.gap, Gap::NoBoardSeen);
    // A leg of another combat is never used.
    let mut r = begun_report();
    r.opening.iter_mut().for_each(|l| l.round = 3);
    assert_eq!(
        odds_input(&r, live(4)).expect_err("not this combat").gap,
        Gap::NoBoardSeen
    );
}

#[test]
fn each_board_plays_with_its_heros_powers_and_trinkets() {
    let lock = HeroKit {
        powers: vec![LogKitCard {
            card_id: "BG22_HERO_000p_Alt".into(),
            counters: [1, 0, 0, 0, 0],
            target: Some(2),
            held: Some(minion("HELD", 5, 6, 0)),
            ..Default::default()
        }],
        trinkets: vec![LogKitCard {
            card_id: "BG30_Trinket_2nd".into(),
            slot: 2,
            ..Default::default()
        }],
    };
    let mut r = solo_report();
    r.rounds[1].entries[0].kit = Some(lock.clone());
    r.rounds[1].entries[1].kit = Some(lock);
    let i = odds_input(&r, live(4)).expect("odds");
    let mut missing = Vec::new();
    let (player, foe) = teams(&i, &facts, &mut missing);
    assert!(missing.is_empty());
    let mine = player.first.kit.expect("the player's kit");
    assert_eq!(mine.round, Some(4));
    assert_eq!(mine.powers[0].counters, [1, 0, 0, 0, 0]);
    assert_eq!(
        mine.powers[0].target,
        Some(1),
        "board position 2 is index 1"
    );
    let held = mine.powers[0].held.as_ref().expect("the held minion");
    assert_eq!((held.attack, held.health), (5, 6));
    assert_eq!(mine.trinkets[0].slot, 2);
    let theirs = foe.first.kit.expect("the opponent's kit");
    assert_eq!(theirs.powers[0].held, None, "never one the opponent holds");

    // A held minion the card data lacks is named like a board's.
    let lacking = |id: &str| if id == "HELD" { None } else { facts(id) };
    let o = run_odds(&i, &lacking, Duration::from_secs(5));
    assert_eq!(o.gap, Some(Gap::NoCardData));
    assert_eq!(o.missing, ["HELD"]);
}

#[test]
fn duos_estimate_waits_for_every_boards_start_and_reads_the_boards_before_it() {
    // Combat 5: the first board of each team has reached its first attack
    // (with its board before start of combat), the player's has begun.
    let mut c5 = duos_round(5, &[(Side::Own, 3, 7), (Side::Opponent, 5, 3)]);
    c5.entries[0].before = Some(plain(1));
    c5.entries[1].before = Some(plain(2));
    let mut r = duos_report(vec![c5]);
    r.opening = vec![opening(5, Side::Own, 2, plain(4), HeroKit::default())];
    let no = odds_input(&r, live(5)).expect_err("seat 6's board has not begun");
    assert_eq!(no.gap, Gap::MissingBoard);

    r.opening
        .push(opening(5, Side::Opponent, 6, plain(6), kit(FIRE)));
    let i = odds_input(&r, live(5)).expect("an estimate");
    assert!(i.estimate);
    assert_eq!((i.own[0].atk, i.opponent[0].atk), (4, 2));
    let d = i.duos.clone().expect("Duos");
    assert!(!d.local_first);
    assert_eq!(d.teammate.board[0].atk, 1);
    assert_eq!(d.opponent_teammate.board[0].atk, 6);
    assert_eq!(d.opponent_teammate.kit, Some(kit(FIRE)));
    let o = run_odds(&i, &facts, Duration::from_secs(5));
    assert_eq!(o.gap, None);
    assert!(o.estimate);
}
