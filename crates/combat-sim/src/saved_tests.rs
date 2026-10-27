//! Hand-made saved games: a replay rebuilt into logged steps, and each
//! combat's odds and cause read from the report.

use serde_json::json;

use super::*;
use crate::combat::replay::Rule;
use crate::why::{Cause, NoReplay, NotSeen, Said, Unseen};

fn steps_of(replay: &Value) -> Option<Vec<LoggedStep>> {
    saved_steps(replay, &|m| minion(m, &facts))
}

fn m(side: &str, card: &str, atk: i64, health: i64, position: i64) -> Value {
    json!({"side": side, "card_id": card, "golden": false, "atk": atk, "health": health,
           "position": position, "keywords": []})
}

#[test]
fn a_saved_replay_gives_the_boards_before_each_attack() {
    let replay = json!({
        "complete": true,
        "start": 3,
        "minions": [
            m("own", "BGS_119", 2, 4, 1),
            m("own", "BG28_300", 1, 1, 2),
            m("opponent", "BGS_119", 1, 5, 1),
            // A Reborn copy with no slot yet: where the same card died.
            m("own", "BG28_300", 1, 1, 0),
        ],
        "events": [
            {"kind": "attack", "attacker": 1, "defender": 2},
            {"kind": "damage", "minion": 2, "amount": 1, "health": 4},
            {"kind": "damage", "minion": 1, "amount": 1, "health": 0},
            {"kind": "death", "minion": 1},
            {"kind": "summon", "minion": 3},
            {"kind": "shield", "minion": 0, "on": true},
            {"kind": "attack", "attacker": 2, "defender": 0},
        ],
    });
    let steps = steps_of(&replay).expect("readable");
    assert_eq!(steps.len(), 2);
    assert_eq!(steps[0].attacker, (0, 1));
    assert_eq!(steps[0].defender, Some(0));
    assert!(!steps[0].all_keywords);
    let second = &steps[1];
    assert_eq!(second.attacker, (1, 0));
    let own: Vec<(&str, i32, i32)> = second.before[0]
        .iter()
        .map(|m| (m.card_id.as_str(), m.attack, m.health))
        .collect();
    assert_eq!(own, [("BGS_119", 2, 4), ("BG28_300", 1, 1)]);
    assert!(second.before[0][0]
        .keywords
        .contains(Keywords::DIVINE_SHIELD));
    assert_eq!(second.before[1][0].health, 4);
}

#[test]
fn an_incomplete_or_broken_replay_is_not_read() {
    let incomplete = json!({"complete": false, "start": 0, "minions": [], "events": []});
    assert!(steps_of(&incomplete).is_none());
    let unknown_minion = json!({
        "complete": true, "start": 1,
        "minions": [m("own", "BGS_119", 1, 1, 1)],
        "events": [{"kind": "attack", "attacker": 5, "defender": null}],
    });
    assert!(steps_of(&unknown_minion).is_none());
}

/// A card with no combat text (`cards::NO_COMBAT_EFFECT`).
const PLAIN: &str = "BG28_603t";

fn facts(id: &str) -> Option<CardFacts> {
    id.starts_with("BG").then_some(CardFacts {
        tier: Some(1),
        tribes: Tribes::NONE,
        base: None,
    })
}

fn bm(card: &str, atk: i64, health: i64, position: i64) -> Value {
    json!({"card_id": card, "golden": false, "atk": atk, "health": health,
           "position": position, "keywords": []})
}

/// The player's 3/3 and `extra` against a 1/1: the player attacks first
/// and always wins.
fn entries(extra: Value) -> Value {
    json!([
        {"side": "own", "player_id": 1, "board": [bm(PLAIN, 3, 3, 1), extra]},
        {"side": "opponent", "player_id": 2, "board": [bm(PLAIN, 1, 1, 1)]},
    ])
}

/// A saved replay of those boards where the opponent's 1/1 attacks first.
fn opponent_first() -> Value {
    let on = |side: &str, mut m: Value| {
        m["side"] = json!(side);
        m
    };
    json!({
        "complete": true,
        "start": 3,
        "minions": [
            on("own", bm(PLAIN, 3, 3, 1)),
            on("own", bm(PLAIN, 1, 1, 2)),
            on("opponent", bm(PLAIN, 1, 1, 1)),
        ],
        "events": [{"kind": "attack", "attacker": 2, "defender": 0}],
    })
}

fn game(game_type: &str, rounds: Value) -> Value {
    json!({"game_type": game_type, "rounds": rounds})
}

fn whys(report: &Value) -> Vec<CombatWhy> {
    let options = Options {
        trials: 100,
        ..Options::default()
    };
    explain_game(report, &facts, &options)
}

fn cause(w: &CombatWhy) -> &Cause {
    match &w.why {
        Why::Explained(e) => e.cause.as_ref().expect("odds"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn each_combat_with_a_result_gets_its_odds_and_cause() {
    let report = game(
        "GT_BATTLEGROUNDS",
        json!([
            {"number": 1, "result": "won", "entries": entries(bm(PLAIN, 1, 1, 2))},
            {"number": 2, "result": "lost", "entries": entries(bm(PLAIN, 1, 1, 2))},
            {"number": 3, "result": "lost", "entries": entries(bm(PLAIN, 1, 1, 2)),
             "replay": opponent_first()},
            // No result: left out.
            {"number": 4, "entries": entries(bm(PLAIN, 1, 1, 2))},
        ]),
    );
    let w = whys(&report);
    assert_eq!(w.iter().map(|w| w.round).collect::<Vec<_>>(), [1, 2, 3]);
    assert_eq!(cause(&w[0]), &Cause::Matched);
    let Why::Explained(e) = &w[0].why else {
        unreachable!()
    };
    assert!(matches!(e.said, Said::Odds(p) if p.won == 1.0));
    assert_eq!(cause(&w[1]).id(), "no_replay_exact");
    let Cause::RuleDifference(d) = cause(&w[2]) else {
        panic!("{:?}", w[2])
    };
    assert_eq!((d.attack, d.rule), (1, Rule::FirstAttacker));
}

#[test]
fn a_replay_whose_first_boards_are_not_the_reports_is_unreadable() {
    let report = game(
        "GT_BATTLEGROUNDS",
        json!([{"number": 2, "result": "lost", "entries": entries(bm(PLAIN, 2, 2, 2)),
                "replay": opponent_first()}]),
    );
    assert!(matches!(
        cause(&whys(&report)[0]),
        Cause::NoReplay {
            why: NoReplay::Unreadable,
            ..
        }
    ));
}

#[test]
fn values_and_unnamed_gifts_come_from_the_report() {
    // Blood Gems with no upgrades saved: missing inputs.
    let gems = "BG33_886";
    let mut gifted = bm(PLAIN, 1, 1, 2);
    gifted["gift_unknown"] = json!(true);
    let mut gem_replay = opponent_first();
    gem_replay["minions"][1]["card_id"] = json!(gems);
    let report = game(
        "GT_BATTLEGROUNDS",
        json!([
            {"number": 1, "result": "lost", "entries": entries(bm(gems, 1, 1, 2)),
             "replay": gem_replay.clone()},
            {"number": 2, "result": "lost", "entries": entries(gifted),
             "replay": opponent_first()},
        ]),
    );
    let w = whys(&report);
    let Cause::MissingInputs { inputs, .. } = cause(&w[0]) else {
        panic!("{:?}", w[0])
    };
    assert_eq!(inputs[0].what, Unseen::BloodGems);
    let Cause::MissingInputs { inputs, .. } = cause(&w[1]) else {
        panic!("{:?}", w[1])
    };
    let gift = NotSeen {
        what: Unseen::Gift,
        card: PLAIN.into(),
    };
    assert_eq!(inputs, &vec![gift]);
    // With the upgrades the board saved, the card is exact.
    let mut e = entries(bm(gems, 1, 1, 2));
    e[0]["values"] = json!({"blood_gem_atk": 2, "blood_gem_health": 2});
    let report = game(
        "GT_BATTLEGROUNDS",
        json!([{"number": 1, "result": "lost", "entries": e, "replay": gem_replay}]),
    );
    assert!(matches!(cause(&whys(&report)[0]), Cause::RuleDifference(_)));
}

#[test]
fn a_combat_without_boards_or_card_data_gets_no_odds() {
    let report = game(
        "GT_BATTLEGROUNDS",
        json!([
            {"number": 1, "result": "won", "entries": [
                {"side": "own", "player_id": 1, "board": [bm(PLAIN, 3, 3, 1)]},
                {"side": "opponent", "player_id": 2, "board": null},
            ]},
            {"number": 2, "result": "won", "entries": entries(bm("UNKNOWN_1", 1, 1, 2))},
        ]),
    );
    let w = whys(&report);
    assert_eq!(w[0].why, Why::NoBoards);
    assert_eq!(w[1].why, Why::NoCardData(vec!["UNKNOWN_1".into()]));
}

#[test]
fn duos_teams_bring_each_seats_board_once_in_order() {
    let rounds = json!([{"number": 3, "result": "won", "entries": [
        {"side": "own", "player_id": 1, "board": [bm(PLAIN, 1, 1, 1)]},
        {"side": "opponent", "player_id": 5, "board": [bm(PLAIN, 1, 1, 1)]},
        {"side": "own", "player_id": 2, "board": [bm(PLAIN, 9, 9, 1)]},
        // The first board again (the hero switching back): left out.
        {"side": "own", "player_id": 1, "board": [bm(PLAIN, 1, 1, 1)]},
    ]}]);
    // The teammate's 9/9 comes in after the two 1/1s trade: a sure win.
    let w = whys(&game("GT_BATTLEGROUNDS_DUO", rounds.clone()));
    assert_eq!(cause(&w[0]), &Cause::Matched);
    // Solo takes one board a side only.
    let solo = whys(&game("GT_BATTLEGROUNDS", rounds));
    assert_eq!(solo[0].why, Why::NoBoards);
}

#[test]
fn a_game_that_is_not_battlegrounds_gives_nothing() {
    let rounds = json!([{"number": 1, "result": "won", "entries": entries(bm(PLAIN, 1, 1, 2))}]);
    assert!(whys(&game("GT_RANKED", rounds.clone())).is_empty());
    assert!(whys(&json!({ "rounds": rounds })).is_empty());
}

#[cfg(feature = "harness")]
#[test]
fn the_kept_values_match_the_parsers_bound() {
    assert_eq!(MAX_GAME_VALUE, bg_parser::report::MAX_GAME_VALUE);
}

#[test]
fn a_saved_kit_is_read_with_its_numbers_target_and_held_minion() {
    let entry = json!({
        "side": "own", "board": [m("own", "BGS_119", 2, 4, 1)],
        "kit": {
            "powers": [{"card_id": "BG22_HERO_000p_Alt", "slot": 0, "counters": [1, 0, 0, 0, 0],
                        "held": {"card_id": "BGS_119", "atk": 6, "health": 10, "golden": false,
                                 "keywords": ["taunt"]}},
                       {"card_id": "BG25_HERO_103p", "slot": 1, "target": 1}],
            "trinkets": [{"card_id": "BG30_MagicItem_902", "slot": 1}]
        }
    });
    let kit = kit(&entry, Some(9), &facts).expect("a kit");
    assert_eq!(kit.round, Some(9));
    assert_eq!(kit.powers[0].counters, [1, 0, 0, 0, 0]);
    let held = kit.powers[0].held.as_ref().expect("held");
    assert_eq!((held.attack, held.health), (6, 10));
    assert!(held.keywords.contains(Keywords::TAUNT));
    assert_eq!(kit.powers[1].target, Some(0));
    assert_eq!(kit.trinkets[0].slot, 1);
    // No kit saved (older parser, or a side not saved): not recorded.
    assert_eq!(
        super::kit(&json!({"side": "opponent", "board": []}), None, &facts),
        None
    );
}

#[test]
fn a_saved_kit_unlike_what_the_parser_writes_is_unknown() {
    let with = |k: Value| json!({"side": "own", "board": [], "kit": k});
    for bad in [
        json!({"powers": []}),
        json!({"powers": [{"slot": 0}], "trinkets": []}),
        json!({"powers": [{"card_id": "X", "slot": 9}], "trinkets": []}),
        json!({"powers": [{"card_id": "X", "slot": 0, "counters": [1]}], "trinkets": []}),
        json!({"powers": [{"card_id": "X", "slot": 0, "counters": [-1, 0, 0, 0, 0]}], "trinkets": []}),
        json!({"powers": [{"card_id": "X", "slot": 0, "target": 8}], "trinkets": []}),
        json!({"powers": [], "trinkets": [{"card_id": "X", "slot": 1, "target": 1}]}),
        json!({"powers": vec![json!({"card_id": "X", "slot": 0}); 5], "trinkets": []}),
    ] {
        assert_eq!(kit(&with(bad.clone()), None, &facts), None, "{bad}");
    }
    // The parser never saves a minion the opponent holds (D-164).
    let held =
        json!({"card_id": "BGS_119", "atk": 1, "health": 1, "golden": false, "keywords": []});
    let opponent = json!({"side": "opponent", "board": [],
        "kit": {"powers": [{"card_id": "X", "slot": 0, "held": held}], "trinkets": []}});
    assert_eq!(kit(&opponent, None, &facts), None);
    // A kit with no board is not one the parser writes.
    let no_board = json!({"side": "own", "board": null, "kit": {"powers": [], "trinkets": []}});
    assert_eq!(kit(&no_board, None, &facts), None);
}

#[test]
fn a_saved_phylactery_buff_is_its_mark_only_on_a_board_whose_kit_has_the_power() {
    let marked = json!({"card_id": "BGS_119", "golden": false, "atk": 2, "health": 2,
        "position": 1, "keywords": [], "buffs": [{"card_id": "BG20_HERO_282", "count": 1}]});
    let other = json!({"card_id": "BGS_119", "golden": false, "atk": 2, "health": 2,
        "position": 2, "keywords": [], "buffs": [{"card_id": "BG20_GEM", "count": 3}]});
    let entry = |powers: Value| {
        json!({"side": "own", "board": [marked, other],
        "kit": {"powers": powers, "trinkets": []}})
    };
    let with_power = entry(json!([{"card_id": "BG20_HERO_282p", "slot": 0}]));
    let b = board(&with_power, &facts).unwrap();
    assert_eq!(b.minions[0].gifts, [crate::gifts::PHYLACTERY_MARK]);
    assert!(b.minions[1].gifts.is_empty());
    // The same buff on a board whose hero has another power, or no saved
    // kit: not the mark.
    for e in [
        entry(json!([{"card_id": "TB_BaconShop_HP_001", "slot": 0}])),
        json!({"side": "own", "board": [marked]}),
    ] {
        assert!(board(&e, &facts).unwrap().minions[0].gifts.is_empty());
    }
}

#[test]
fn board_minions_take_the_numbers_the_replay_start_shows() {
    let with = |mut m: Value, side: Option<&str>, counters: Option<[i64; 2]>| {
        if let Some(s) = side {
            m["side"] = json!(s);
        }
        if let Some(c) = counters {
            m["counters"] = json!(c);
        }
        m
    };
    let lobster = "BG36_202";
    let round = json!({
        "number": 1,
        "result": "won",
        "entries": [
            {"side": "own", "player_id": 1,
             "board": [bm(lobster, 2, 1, 1), bm(PLAIN, 3, 3, 2), bm(lobster, 2, 1, 3)]},
            {"side": "opponent", "player_id": 2, "board": [bm(lobster, 2, 1, 1)]},
        ],
        "replay": {
            "complete": true,
            "start": 4,
            "minions": [
                with(bm(lobster, 2, 1, 1), Some("own"), None),
                with(bm(PLAIN, 3, 3, 2), Some("own"), Some([5, 5])),
                with(bm(lobster, 2, 1, 3), Some("own"), Some([18, 9])),
                with(bm(lobster, 2, 1, 1), Some("opponent"), Some([4, 2])),
                // Summoned later: not the start's.
                with(bm(lobster, 2, 1, 2), Some("opponent"), Some([2, 1])),
            ],
            "events": [],
        },
    });
    let Ok(Prepared { teams, .. }) = prepare(&round, true, &facts) else {
        panic!("boards");
    };
    let numbers = |b: &Warband| b.minions.iter().map(|m| m.counters).collect::<Vec<_>>();
    // Only Lobsters take them.
    assert_eq!(numbers(&teams.0.first), [[0, 0], [0, 0], [18, 9]]);
    assert_eq!(numbers(&teams.1.first), [[4, 2]]);
}

#[test]
fn without_a_replay_board_minions_have_no_numbers() {
    let round = json!({"number": 1, "result": "won", "entries": entries(bm("BG36_202", 2, 1, 2))});
    let Ok(Prepared { teams, .. }) = prepare(&round, true, &facts) else {
        panic!("boards");
    };
    assert!(teams.0.first.minions.iter().all(|m| m.counters == [0, 0]));
}

#[test]
fn a_saved_rounds_boards_bring_their_kits() {
    // T-645: the powers and trinkets act in the saved combats as in the
    // overlay (Sprout It Out on the minions a Greybough summons); a board
    // without a saved kit plays as before.
    let sprout = json!({"powers": [{"card_id": "TB_BaconShop_HP_107", "slot": 0}],
                        "trinkets": []});
    let round = json!({"number": 6, "result": "won", "entries": [
        {"side": "own", "player_id": 1, "board": [bm(PLAIN, 3, 3, 1)], "kit": sprout},
        {"side": "opponent", "player_id": 2, "board": [bm(PLAIN, 1, 1, 1)]},
        {"side": "own", "player_id": 3, "board": [bm(PLAIN, 2, 2, 1)], "kit": sprout},
    ]});
    let Ok(Prepared { teams, .. }) = prepare(&round, false, &facts) else {
        panic!("boards");
    };
    for band in [&teams.0.first, teams.0.second.as_ref().expect("teammate")] {
        let kit = band.kit.as_ref().expect("a kit");
        assert_eq!(kit.round, Some(6));
        assert_eq!(kit.powers[0].card_id, "TB_BaconShop_HP_107");
    }
    assert_eq!(teams.1.first.kit, None);
}

/// One Solo round won by the player's 3/3, its board with `kit`.
fn round_with_kit(kit: Value) -> Value {
    game(
        "GT_BATTLEGROUNDS",
        json!([{"number": 2, "result": "won", "entries": [
            {"side": "own", "player_id": 1, "board": [bm(PLAIN, 3, 3, 1)], "kit": kit},
            {"side": "opponent", "player_id": 2, "board": [bm(PLAIN, 1, 1, 1)]},
        ]}]),
    )
}

#[test]
fn a_saved_kit_that_cannot_be_read_refuses_the_boards() {
    // T-645: never played as if the board had no powers or trinkets.
    let bad = json!({"powers": [{"card_id": "TB_BaconShop_HP_107", "slot": 9}],
                     "trinkets": []});
    let w = whys(&round_with_kit(bad));
    let Why::Explained(e) = &w[0].why else {
        panic!("{:?}", w[0])
    };
    assert_eq!(e.said, Said::BadBoard);
    assert_eq!(e.cause, None);
    // A record without a kit ("not recorded") plays as before.
    let w = whys(&round_with_kit(Value::Null));
    assert_eq!(cause(&w[0]), &Cause::Matched);
}

/// A Teron's full board taken after start of combat: Rapid Reanimation
/// destroyed its second minion (a 41/1 Eternal Summoner, 42/2 when it was
/// destroyed) and an Eternal Knight took the place; `extra` events come
/// before the first attack.
fn teron_round(kit: Value, extra: &[Value]) -> Value {
    let on = |side: &str, mut m: Value| {
        m["side"] = json!(side);
        m
    };
    let after: Vec<Value> = (1..=7)
        .map(|p| bm(if p == 2 { "BG25_008" } else { PLAIN }, 1, 1, p))
        .collect();
    let mut before: Vec<Value> = (1..=7)
        .map(|p| on("own", bm(if p == 2 { "BG25_009" } else { PLAIN }, 41, 1, p)))
        .collect();
    before.push(on("opponent", bm(PLAIN, 1, 1, 1)));
    let mut minions = before;
    minions.push(on("own", bm("BG25_008", 1, 1, 2)));
    minions.push(on("own", bm("BG25_009", 42, 1, 2)));
    let mut events = vec![
        json!({"kind": "trigger", "card_id": "BG25_HERO_103p", "side": "own",
               "trigger": "start_of_combat", "source": null}),
        json!({"kind": "stats", "minion": 1, "atk": 42, "health": 2}),
        json!({"kind": "death", "minion": 1}),
        json!({"kind": "summon", "minion": 8}),
    ];
    events.extend(extra.iter().cloned());
    events.push(json!({"kind": "attack", "attacker": 0, "defender": 7}));
    json!({"number": 8, "result": "won",
        "entries": [
            {"side": "own", "player_id": 1, "board": after, "kit": kit},
            {"side": "opponent", "player_id": 2, "board": [bm(PLAIN, 1, 1, 1)]},
        ],
        "replay": {"complete": true, "start": 8, "minions": minions, "events": events},
    })
}

fn rapid_reanimation(target: i64) -> Value {
    json!({"powers": [{"card_id": "BG25_HERO_103p", "slot": 0, "target": target}],
           "trinkets": []})
}

fn held_copy(round: &Value) -> Option<Minion> {
    let Ok(Prepared { teams, .. }) = prepare(round, true, &facts) else {
        panic!("boards");
    };
    let kit = teams.0.first.kit.expect("a kit");
    kit.powers[0].held.clone()
}

#[test]
fn rapid_reanimations_copy_waiting_for_space_is_read_from_the_replay_start() {
    // T-645: the board at the first attack is full; the copy, as the
    // Summoner was when destroyed, waits for space.
    let copy = held_copy(&teron_round(rapid_reanimation(2), &[])).expect("waiting");
    assert_eq!(
        (copy.card_id.as_str(), copy.attack, copy.health),
        ("BG25_009", 42, 2)
    );
    // The copy came back before the first attack: nothing waits.
    let back = json!({"kind": "summon", "minion": 9});
    assert_eq!(held_copy(&teron_round(rapid_reanimation(2), &[back])), None);
    // The named minion did not die at the start of combat: nothing waits.
    assert_eq!(held_copy(&teron_round(rapid_reanimation(3), &[])), None);
    // It died, but the power never acted: not its copy.
    let mut silent = teron_round(rapid_reanimation(2), &[]);
    silent["replay"]["events"]
        .as_array_mut()
        .expect("events")
        .remove(0);
    assert_eq!(held_copy(&silent), None);
    // A record without a kit stays as it is.
    let Ok(Prepared { teams, .. }) = prepare(&teron_round(Value::Null, &[]), true, &facts) else {
        panic!("boards");
    };
    assert_eq!(teams.0.first.kit, None);
}

fn events(round: &mut Value) -> &mut Vec<Value> {
    round["replay"]["events"].as_array_mut().expect("events")
}

fn replay_minions(round: &mut Value) -> &mut Vec<Value> {
    round["replay"]["minions"].as_array_mut().expect("minions")
}

/// The copy waiting on the player's first board's kit (`side` 0) or the
/// opponent's.
fn held_on(round: &Value, side: usize) -> Option<Minion> {
    let Ok(Prepared { teams, .. }) = prepare(round, true, &facts) else {
        panic!("boards");
    };
    let band = if side == 0 {
        teams.0.first
    } else {
        teams.1.first
    };
    band.kit.expect("a kit").powers[0].held.clone()
}

fn stats_of(m: Option<Minion>) -> Option<(i32, i32)> {
    m.map(|m| (m.attack, m.health))
}

#[test]
fn rapid_reanimation_waits_only_for_a_minion_it_destroyed_on_its_side() {
    // The named minion died before the power acted (to a trinket).
    let mut early = teron_round(rapid_reanimation(2), &[]);
    let trigger = events(&mut early).remove(0);
    events(&mut early).insert(2, trigger);
    assert_eq!(held_copy(&early), None);
    // The power that acted is the opponent's.
    let mut theirs = teron_round(rapid_reanimation(2), &[]);
    events(&mut theirs)[0]["side"] = json!("opponent");
    assert_eq!(held_copy(&theirs), None);
    // No minion before start of combat at the named place.
    let mut nowhere = teron_round(rapid_reanimation(2), &[]);
    replay_minions(&mut nowhere)[1]["position"] = Value::Null;
    assert_eq!(held_copy(&nowhere), None);
}

#[test]
fn an_opponents_rapid_reanimation_waits_too() {
    let mut round = teron_round(rapid_reanimation(2), &[]);
    let swap = |v: &mut Value| {
        let side = match v["side"].as_str() {
            Some("own") => "opponent",
            Some("opponent") => "own",
            _ => return,
        };
        v["side"] = json!(side);
    };
    round["entries"]
        .as_array_mut()
        .expect("entries")
        .iter_mut()
        .for_each(swap);
    replay_minions(&mut round).iter_mut().for_each(swap);
    events(&mut round).iter_mut().for_each(swap);
    assert_eq!(stats_of(held_on(&round, 1)), Some((42, 2)));
}

#[test]
fn only_summons_of_its_card_after_its_death_bring_the_copy_back() {
    let summon = |i: usize| json!({"kind": "summon", "minion": i});
    // A Summoner summoned before it died is another one: the copy waits.
    let mut earlier = teron_round(rapid_reanimation(2), &[]);
    events(&mut earlier).insert(0, summon(9));
    assert_eq!(stats_of(held_copy(&earlier)), Some((42, 2)));
    // With Reborn, its first summon after the death is its Reborn.
    let mut reborn = teron_round(rapid_reanimation(2), &[summon(9)]);
    replay_minions(&mut reborn)[1]["keywords"] = json!(["reborn"]);
    assert_eq!(stats_of(held_copy(&reborn)), Some((42, 2)));
    // A second summon of its card is the copy.
    replay_minions(&mut reborn).push(json!({"side": "own", "card_id": "BG25_009",
        "golden": false, "atk": 42, "health": 2, "position": 3, "keywords": []}));
    events(&mut reborn).insert(5, summon(10));
    assert_eq!(held_copy(&reborn), None);
}

#[test]
fn the_waiting_copy_keeps_its_card_and_falls_back_on_the_start_stats() {
    // Golden stays golden.
    let mut golden = teron_round(rapid_reanimation(2), &[]);
    replay_minions(&mut golden)[1]["golden"] = json!(true);
    assert!(held_copy(&golden).expect("waiting").golden);
    // No stats event before its death: as the replay start shows it.
    let mut plain = teron_round(rapid_reanimation(2), &[]);
    events(&mut plain).remove(1);
    assert_eq!(stats_of(held_copy(&plain)), Some((41, 1)));
    // The named minion found by side and place, wherever it is listed.
    let mut shifted = teron_round(rapid_reanimation(2), &[]);
    replay_minions(&mut shifted).insert(
        0,
        json!({"side": "opponent", "card_id": PLAIN, "golden": false, "atk": 1,
               "health": 1, "position": 2, "keywords": []}),
    );
    shifted["replay"]["start"] = json!(9);
    for e in events(&mut shifted).iter_mut() {
        for key in ["minion", "attacker", "defender"] {
            if let Some(i) = e[key].as_u64() {
                e[key] = json!(i + 1);
            }
        }
    }
    assert_eq!(stats_of(held_copy(&shifted)), Some((42, 2)));
}

#[test]
fn no_copy_waits_without_a_complete_replay_or_a_full_board() {
    let mut incomplete = teron_round(rapid_reanimation(2), &[]);
    incomplete["replay"]["complete"] = json!(false);
    assert_eq!(held_copy(&incomplete), None);
    // With space at the first attack the copy would be back already.
    let mut space = teron_round(rapid_reanimation(2), &[]);
    space["entries"][0]["board"]
        .as_array_mut()
        .expect("board")
        .pop();
    assert_eq!(held_copy(&space), None);
}

#[test]
fn an_unmodelled_trinket_on_a_saved_board_blocks_the_odds() {
    let trinket = json!({"powers": [],
                         "trinkets": [{"card_id": "BG99_MagicItem_1", "slot": 1}]});
    let w = whys(&round_with_kit(trinket));
    let Why::Explained(e) = &w[0].why else {
        panic!("{:?}", w[0])
    };
    assert_eq!(e.said, Said::NotModelled(vec!["BG99_MagicItem_1".into()]));
}
