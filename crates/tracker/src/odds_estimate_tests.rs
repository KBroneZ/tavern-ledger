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
    assert!(
        o.start_of_combat,
        "its start of combat is already on the boards, so the odds are marked (T-632)"
    );

    // Once in, they win over a leg still listed as begun.
    r.opening = begun_report().opening;
    assert!(!odds_input(&r, live(4)).expect("odds").estimate);
}

/// Holy Mallet, a trinket that acts at the start of combat.
const MALLET: &str = "BG30_MagicItem_902";
/// Bleeding Heart: no effect in combat. Bronzebeard Portrait: acts later in it.
const NO_EFFECT: &str = "BG30_MagicItem_713";
const LATER: &str = "BG30_MagicItem_418";

fn with_trinket(id: &str) -> HeroKit {
    HeroKit {
        powers: Vec::new(),
        trinkets: vec![LogKitCard {
            card_id: id.into(),
            slot: 1,
            ..Default::default()
        }],
    }
}

/// Combat 4 at the first attack, the player with `kit`.
fn first_attack_with(kit: HeroKit) -> GameReport {
    let mut r = report(GAME, vec![round(3, Some(plain(1)), 5, Some(plain(9)))]);
    let mut fought = round(
        4,
        Some(vec![minion(PLAIN, 2, 3, 1)]),
        5,
        Some(vec![minion(PLAIN, 3, 3, 1)]),
    );
    fought.entries[0].kit = Some(kit);
    r.rounds.push(fought);
    r
}

#[test]
fn a_hero_power_or_trinket_with_start_of_combat_marks_the_first_attacks_odds() {
    // The mark is one rule for every source: a minion, a Dark Gift, a hero
    // power or a trinket that acts at the start of combat (T-632).
    for (kit, marked) in [
        (kit(FIRE), true),
        (with_trinket(MALLET), true),
        (with_trinket(NO_EFFECT), false),
        (with_trinket(LATER), false),
        (HeroKit::default(), false),
    ] {
        let r = first_attack_with(kit.clone());
        let i = odds_input(&r, live(4)).expect("odds");
        assert_eq!(i.seen_after_start_of_combat, marked, "log side: {kit:?}");
        let mut by_boards = i.clone();
        by_boards.seen_after_start_of_combat = false;
        let o = run_odds(&by_boards, &facts, Duration::from_secs(5));
        assert_eq!(o.start_of_combat, marked, "boards side: {kit:?}");
    }
}

#[test]
fn the_estimate_is_never_marked_for_a_source_it_plays() {
    let i = odds_input(&begun_report(), live(4)).expect("an estimate");
    assert!(!run_odds(&i, &facts, Duration::from_secs(5)).start_of_combat);
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

/// A minion with no combat effect, held by a hero power.
const HELD: &str = "BG26_361";

/// Lock and Load holding a 5/6 [`HELD`], and the greater trinket slot's
/// placeholder.
fn lock_and_load() -> HeroKit {
    HeroKit {
        powers: vec![LogKitCard {
            card_id: "BG22_HERO_000p_Alt".into(),
            counters: [1, 0, 0, 0, 0],
            target: Some(2),
            held: Some(minion(HELD, 5, 6, 0)),
            ..Default::default()
        }],
        trinkets: vec![LogKitCard {
            card_id: "BG30_Trinket_2nd".into(),
            slot: 2,
            ..Default::default()
        }],
    }
}

/// Card data that lacks [`HELD`].
fn lacking(id: &str) -> Option<Facts> {
    if id == HELD {
        None
    } else {
        facts(id)
    }
}

#[test]
fn each_board_plays_with_its_heros_powers_and_trinkets() {
    let mut r = solo_report();
    r.rounds[1].entries[0].kit = Some(lock_and_load());
    r.rounds[1].entries[1].kit = Some(lock_and_load());
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
    assert_eq!(mine.trinkets[0].slot, 2);
    assert!(foe.first.kit.is_some(), "the opponent's kit");
    // At the first attack a held minion never comes in (it does only with
    // start of combat played), so the card data it lacks does not matter.
    assert_eq!(mine.powers[0].held, None);
    assert_eq!(run_odds(&i, &lacking, Duration::from_secs(5)).gap, None);
}

#[test]
fn an_estimate_brings_in_the_players_held_minion_never_the_opponents() {
    let mut r = begun_report();
    r.opening[0].kit = lock_and_load();
    r.opening[1].kit = lock_and_load();
    let i = odds_input(&r, live(4)).expect("an estimate");
    let mut missing = Vec::new();
    let (player, foe) = teams(&i, &facts, &mut missing);
    let held = player.first.kit.expect("the player's kit").powers[0]
        .held
        .clone()
        .expect("the held minion");
    assert_eq!((held.attack, held.health), (5, 6));
    let theirs = foe.first.kit.expect("the opponent's kit");
    assert_eq!(
        theirs.powers[0].held, None,
        "never one the opponent holds (D-163)"
    );
    // Without it the opponent's Lock and Load cannot be played: no number.
    let o = run_odds(&i, &facts, Duration::from_secs(5));
    assert_eq!(o.gap, Some(Gap::NoBoardSeen));

    // The player's alone is played; a held minion the card data lacks is
    // named like a board's.
    r.opening[1].kit = HeroKit::default();
    let i = odds_input(&r, live(4)).expect("an estimate");
    let o = run_odds(&i, &facts, Duration::from_secs(5));
    assert_eq!((o.gap, o.estimate), (None, true));
    let o = run_odds(&i, &lacking, Duration::from_secs(5));
    assert_eq!(o.gap, Some(Gap::NoCardData));
    assert_eq!(o.missing, [HELD]);
}

#[test]
fn an_estimate_with_a_start_of_combat_power_not_played_gives_no_number_yet() {
    // Embrace Your Rage acts at the start of combat and is not played.
    const RAGE: &str = "TB_BaconShop_HP_103";
    assert!(combat_sim::kit::has_start_of_combat(RAGE));
    assert!(combat_sim::kit_effects::played(RAGE).is_none());
    let mut r = begun_report();
    r.opening[1].kit = kit(RAGE);
    let i = odds_input(&r, live(4)).expect("an estimate's input");
    let o = run_odds(&i, &facts, Duration::from_secs(5));
    assert_eq!(o.gap, Some(Gap::NoBoardSeen));

    // At the first attack its effect is on the boards: odds.
    r.opening.clear();
    let mut fought = round(4, Some(plain(2)), 5, Some(plain(3)));
    fought.entries[1].kit = Some(kit(RAGE));
    r.rounds.push(fought);
    let i = odds_input(&r, live(4)).expect("odds");
    assert_eq!(run_odds(&i, &facts, Duration::from_secs(5)).gap, None);
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

#[test]
fn duos_brings_in_the_teammates_held_minion_like_the_players() {
    // D-167: the screen shows what the teammate's Lock and Load holds.
    // Combat 5: the teammate's board (seat 3) fought first, with Lock and
    // Load; the player's and seat 6's have begun.
    let mut c5 = duos_round(5, &[(Side::Own, 3, 7), (Side::Opponent, 5, 3)]);
    c5.entries[0].before = Some(plain(1));
    c5.entries[1].before = Some(plain(2));
    c5.entries[0].kit = Some(lock_and_load());
    let mut r = duos_report(vec![c5]);
    r.opening = vec![
        opening(5, Side::Own, 2, plain(4), HeroKit::default()),
        opening(5, Side::Opponent, 6, plain(6), lock_and_load()),
    ];
    let i = odds_input(&r, live(5)).expect("an estimate");
    let mut missing = Vec::new();
    let (player, foe) = teams(&i, &facts, &mut missing);
    let held = |b: &Warband| b.kit.as_ref().and_then(|k| k.powers[0].held.clone());
    assert_eq!(
        held(&player.first).map(|m| m.attack),
        Some(5),
        "the teammate's (it fights first)"
    );
    let foe_mate = foe.second.as_ref().expect("two boards");
    assert_eq!(held(foe_mate), None, "never an opponent's");
}
