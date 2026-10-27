use super::*;
use serde_json::json;

fn offer(card: &str, roll: u32, frozen: bool) -> Value {
    json!({"card_id": card, "roll": roll, "frozen": frozen})
}

fn act(ms: i64, turn: i64, kind: &str) -> Value {
    json!({"ms": ms, "turn": turn, "kind": kind})
}

fn turn(n: i64, buys: &[&str], sells: &[&str], offers: Vec<Value>) -> Value {
    json!({
        "turn": n, "start_ms": n * 1000, "end_ms": n * 1000 + 999, "tier": 2, "gold": 5,
        "gold_spent": 5, "rolls": 1, "free_rolls": 0, "buys": buys, "spell_buys": 0,
        "sells": sells, "freezes": 0, "actions": 0, "offers": offers,
    })
}

fn report(turns: Vec<Value>, actions: Vec<Value>, discovers: Vec<Value>) -> Value {
    json!({
        "status": "ok",
        "shop": {
            "turns": turns, "tier_ups": [{"turn": 2, "tier": 3}], "actions": actions,
            "start_ms": 0, "end_ms": 9_000, "ended": true,
        },
        "discovers": discovers,
    })
}

fn kinds(d: &[Decision]) -> Vec<DecisionKind> {
    d.iter().map(|d| d.kind).collect()
}

fn ids(v: &[Option<String>]) -> Vec<&str> {
    v.iter().map(|c| c.as_deref().unwrap_or("?")).collect()
}

/// Turn 1: the first shop shows A and B; the player buys A, rolls (C, D),
/// buys D and freezes what is left.
fn one_turn() -> Value {
    report(
        vec![turn(
            1,
            &["BG_A", "BG_D"],
            &[],
            vec![
                offer("BG_A", 0, false),
                offer("BG_B", 0, false),
                offer("BG_C", 1, false),
                offer("BG_D", 1, false),
            ],
        )],
        vec![
            act(1100, 1, "buy"),
            act(1200, 1, "roll"),
            act(1300, 1, "buy"),
            act(1400, 1, "freeze"),
            act(1500, 1, "move"),
        ],
        vec![],
    )
}

#[test]
fn each_buy_roll_and_freeze_is_a_decision_with_the_shop_it_was_made_in() {
    let d = decisions(&one_turn()).unwrap();

    use DecisionKind::*;
    assert_eq!(kinds(&d), [Buy, Roll, Buy, Freeze]);
    assert_eq!(d[0].taken.as_deref(), Some("BG_A"));
    assert_eq!(ids(&d[0].offered), ["BG_A", "BG_B"]);
    assert!(!d[0].inferred);
    // The roll leaves B behind; the second buy chooses among the new shop.
    assert_eq!(ids(&d[1].offered), ["BG_B"]);
    assert_eq!(ids(&d[2].offered), ["BG_C", "BG_D"]);
    assert_eq!(d[2].taken.as_deref(), Some("BG_D"));
    // The freeze keeps what was left after the buy.
    assert_eq!(ids(&d[3].offered), ["BG_C"]);
    assert_eq!(d[3].ms, Some(1400));
    assert!(d.iter().all(|d| d.turn == 1));
}

#[test]
fn a_frozen_minion_is_in_the_turns_first_shop() {
    let r = report(
        vec![turn(
            2,
            &["BG_F"],
            &[],
            vec![offer("BG_F", 0, true), offer("BG_G", 0, false)],
        )],
        vec![act(2100, 2, "buy")],
        vec![],
    );

    let d = decisions(&r).unwrap();

    assert_eq!(ids(&d[0].offered), ["BG_F", "BG_G"]);
    assert!(!d[0].inferred);
}

#[test]
fn a_buy_of_a_card_the_shop_did_not_show_is_inferred() {
    let r = report(
        vec![turn(1, &["BG_Z"], &[], vec![offer("BG_A", 0, false)])],
        vec![act(1100, 1, "buy")],
        vec![],
    );

    let d = decisions(&r).unwrap();

    assert_eq!(d[0].taken.as_deref(), Some("BG_Z"));
    assert!(d[0].inferred);
}

#[test]
fn when_the_buys_and_the_buy_actions_do_not_agree_every_buy_of_the_turn_is_inferred() {
    let r = report(
        vec![turn(
            1,
            &["BG_A", "BG_B"],
            &[],
            vec![offer("BG_A", 0, false), offer("BG_B", 0, false)],
        )],
        vec![act(1100, 1, "buy")],
        vec![],
    );

    let d = decisions(&r).unwrap();

    let buys: Vec<&Decision> = d.iter().filter(|d| d.kind == DecisionKind::Buy).collect();
    assert_eq!(buys.len(), 2);
    assert!(buys.iter().all(|b| b.inferred));
    assert_eq!(buys[1].ms, None);
}

#[test]
fn sells_name_the_minion_but_not_the_board_and_tier_ups_say_the_tier() {
    let r = report(
        vec![turn(2, &[], &["BG_S"], vec![])],
        vec![act(2100, 2, "sell"), act(2200, 2, "tier_up")],
        vec![],
    );

    let d = decisions(&r).unwrap();

    assert_eq!(d[0].kind, DecisionKind::Sell);
    assert_eq!(d[0].taken.as_deref(), Some("BG_S"));
    assert!(d[0].offered.is_empty());
    assert_eq!(d[1].kind, DecisionKind::TierUp);
    assert_eq!(d[1].tier, Some(3));
}

#[test]
fn a_discover_is_a_choice_among_its_options_with_its_source() {
    let discover = json!({
        "turn": 1, "source": "BG_SRC",
        "offered": [{"card_id": "BG_X", "gift": null}, {"card_id": "BG_Y", "gift": null}],
        "picked": [1],
    });
    let r = report(
        vec![turn(1, &[], &[], vec![])],
        vec![act(1100, 1, "choose")],
        vec![discover],
    );

    let d = decisions(&r).unwrap();

    assert_eq!(d[0].kind, DecisionKind::Choose);
    assert_eq!(d[0].source.as_deref(), Some("BG_SRC"));
    assert_eq!(ids(&d[0].offered), ["BG_X", "BG_Y"]);
    assert_eq!(d[0].taken.as_deref(), Some("BG_Y"));
    assert!(!d[0].inferred);
}

#[test]
fn a_discover_without_its_action_or_without_a_pick_is_inferred() {
    let unpicked = json!({"turn": 1, "source": null, "offered": [{"card_id": "BG_X", "gift": null}], "picked": []});
    let r = report(vec![turn(1, &[], &[], vec![])], vec![], vec![unpicked]);

    let d = decisions(&r).unwrap();

    assert_eq!(d.len(), 1);
    assert_eq!(d[0].ms, None);
    assert_eq!(d[0].taken, None);
    assert!(d[0].inferred);
}

#[test]
fn turns_without_logged_actions_still_give_their_buys_and_sells_inferred() {
    let r = report(
        vec![turn(3, &["BG_A"], &["BG_B"], vec![offer("BG_A", 0, false)])],
        vec![],
        vec![],
    );

    let d = decisions(&r).unwrap();

    assert_eq!(kinds(&d), [DecisionKind::Buy, DecisionKind::Sell]);
    assert!(d.iter().all(|d| d.inferred && d.ms.is_none()));
}

#[test]
fn a_game_without_a_shop_record_says_why() {
    assert_eq!(
        decisions(&json!({"status": "ok"})),
        Err(ShopState::NotRecorded)
    );
    assert_eq!(
        decisions(&json!({"shop": null})),
        Err(ShopState::NotAvailable)
    );
}

#[test]
fn moves_hero_powers_and_cards_played_are_not_shop_decisions() {
    let r = report(
        vec![turn(1, &[], &[], vec![])],
        vec![
            act(1100, 1, "move"),
            act(1200, 1, "hero_power"),
            act(1300, 1, "play"),
        ],
        vec![],
    );

    assert!(decisions(&r).unwrap().is_empty());
}
