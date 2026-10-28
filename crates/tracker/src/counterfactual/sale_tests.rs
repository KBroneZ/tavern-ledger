//! A sale weighed against what its gold paid for later in the turn.

use super::sale::kept_instead;
use super::tests::{
    act, minion, offer, reason, rebuilt, report, round, run, shop_turn, FILLER, FOE, OTHER, STRONG,
    WEAK,
};
use super::{Approx, NotRebuilt};
use crate::decisions::DecisionKind;
use serde_json::{json, Value};

/// Turn 2 sells STRONG (on the board of combat 1), then does `after`
/// (actions after the sale, in order); `buys` are the cards the record
/// says were bought that turn, `own` the board of combat 2.
fn sale_then(after: &[&str], buys: &[&str], own: Vec<Value>) -> Value {
    let mut actions = vec![act(2, 0, "sell")];
    actions.extend(
        after
            .iter()
            .enumerate()
            .map(|(k, kind)| act(2, k + 1, kind)),
    );
    let mut turn = shop_turn(2, buys, &[STRONG], &[WEAK, OTHER, FILLER]);
    turn["rolls"] = json!(after.iter().filter(|k| **k == "roll").count());
    report(
        vec![shop_turn(1, &[], &[], &[]), turn],
        actions,
        vec![
            round(1, vec![minion(STRONG, 20, 20)], vec![minion(FOE, 3, 3)]),
            round(2, own, vec![minion(FOE, 3, 3)]),
        ],
    )
}

#[test]
fn a_sale_followed_by_a_buy_is_weighed_against_skipping_that_buy() {
    let g = sale_then(&["buy"], &[WEAK], vec![minion(WEAK, 1, 1)]);

    let r = run(&g);

    assert_eq!(r.decisions[0].decision.kind, DecisionKind::Sell);
    let b = rebuilt(&r, 0);
    assert_eq!(b.alternatives.len(), 1);
    let alt = &b.alternatives[0];
    assert_eq!(alt.card, STRONG);
    assert_eq!(alt.instead_of.as_deref(), Some(WEAK));
    // Skipping the buy saves more gold than the sale gave: nothing is
    // short, but what the buy did as it was played stays on the board.
    assert_eq!(
        alt.approx,
        [Approx::EarlierStats, Approx::BattlecryNotPlayed]
    );
    // STRONG alone, in place of WEAK: a sure win.
    assert_eq!(alt.fight.as_ref().unwrap().won, 1.0);
    assert_eq!(b.actual.lost, 1.0);
}

#[test]
fn the_skipped_buy_is_the_first_after_the_sale() {
    let g = sale_then(
        &["buy", "buy"],
        &[WEAK, OTHER],
        vec![minion(WEAK, 1, 1), minion(OTHER, 2, 2)],
    );

    let r = run(&g);

    let alt = &rebuilt(&r, 0).alternatives[0];
    assert_eq!(alt.instead_of.as_deref(), Some(WEAK));
}

#[test]
fn a_buy_before_the_sale_leaves_the_plain_keep() {
    let mut g = sale_then(&[], &[WEAK], vec![minion(WEAK, 1, 1)]);
    // The buy comes first in the turn, the sale after it.
    g["shop"]["actions"] = json!([act(2, 0, "buy"), act(2, 1, "sell")]);

    let r = run(&g);

    assert_eq!(r.decisions[1].decision.kind, DecisionKind::Sell);
    let alt = &rebuilt(&r, 1).alternatives[0];
    assert_eq!(alt.instead_of, None);
    assert_eq!(alt.approx, [Approx::EarlierStats]);
}

#[test]
fn a_sale_followed_only_by_gold_spent_otherwise_may_have_been_short() {
    for after in [["roll"], ["tier_up"], ["buy_spell"]] {
        let g = sale_then(&after, &[], vec![minion(WEAK, 1, 1)]);

        let r = run(&g);

        let alt = &rebuilt(&r, 0).alternatives[0];
        assert_eq!(alt.instead_of, None, "{after:?}");
        assert_eq!(
            alt.approx,
            [Approx::EarlierStats, Approx::GoldMaybeShort],
            "{after:?}"
        );
        assert!(alt.fight.is_ok());
    }
}

#[test]
fn a_sale_with_nothing_after_it_keeps_the_plain_keep() {
    // Nothing after it, or only what costs no gold: a freeze, an unfreeze,
    // a pick from a discover.
    for after in [&[][..], &["freeze"], &["unfreeze"], &["choose"]] {
        let g = sale_then(after, &[], vec![minion(WEAK, 1, 1)]);

        let r = run(&g);

        let alt = &rebuilt(&r, 0).alternatives[0];
        assert_eq!(alt.instead_of, None, "{after:?}");
        assert_eq!(alt.approx, [Approx::EarlierStats], "{after:?}");
    }
}

#[test]
fn a_buy_that_cannot_be_found_on_the_board_is_not_guessed() {
    // Two copies of the minion bought: which one was it?
    let g = sale_then(
        &["buy"],
        &[WEAK],
        vec![minion(WEAK, 1, 1), minion(WEAK, 2, 2)],
    );
    assert_eq!(reason(&run(&g), 0), NotRebuilt::Ambiguous);

    // Bought but not on the board (in hand, made golden...).
    let g = sale_then(&["buy"], &[WEAK], vec![minion(OTHER, 2, 2)]);
    assert_eq!(reason(&run(&g), 0), NotRebuilt::NextBuyNotOnBoard);

    // Bought, then sold again.
    let mut g = sale_then(&["buy", "sell"], &[WEAK], vec![minion(WEAK, 1, 1)]);
    g["shop"]["turns"][1]["sells"] = json!([STRONG, WEAK]);
    assert_eq!(reason(&run(&g), 0), NotRebuilt::Ambiguous);

    // A buy the log does not name.
    let g = sale_then(&["buy"], &[], vec![minion(WEAK, 1, 1)]);
    assert_eq!(reason(&run(&g), 0), NotRebuilt::NextBuyNotOnBoard);

    // A buy matched up rather than read as the log has it: a card that
    // shop did not show.
    let mut g = sale_then(&["buy"], &[WEAK], vec![minion(WEAK, 1, 1)]);
    g["shop"]["turns"][1]["offers"] = json!([offer(OTHER), offer(FILLER)]);
    let r = run(&g);
    assert!(r.decisions[1].decision.inferred);
    assert_eq!(reason(&r, 0), NotRebuilt::NextBuyUncertain);
}

#[test]
fn a_buy_without_a_time_leaves_the_order_unknown() {
    // Two buys recorded, one buy action: the second has no time, so it
    // may have come after the sale.
    let mut g = sale_then(
        &[],
        &[WEAK, OTHER],
        vec![minion(WEAK, 1, 1), minion(OTHER, 2, 2)],
    );
    g["shop"]["actions"] = json!([act(2, 0, "buy"), act(2, 1, "sell")]);

    let r = run(&g);

    let sale = r
        .decisions
        .iter()
        .position(|d| d.decision.kind == DecisionKind::Sell)
        .unwrap();
    assert_eq!(reason(&r, sale), NotRebuilt::OrderUnknown);
}

#[test]
fn the_minion_kept_goes_back_where_it_was_and_the_buy_leaves() {
    let own = json!({"board": [
        minion(FILLER, 1, 1), minion(OTHER, 2, 2), minion(WEAK, 1, 1),
    ], "kit": {"powers": [
        {"card_id": "P1", "slot": 0, "target": 1},
        {"card_id": "P2", "slot": 1, "target": 3},
        {"card_id": "P3", "slot": 1, "target": 2},
    ], "trinkets": []}});
    let kept = minion(STRONG, 20, 20);

    // STRONG was second before the sale; OTHER (second now) was bought.
    let out = kept_instead(&own, &kept, 1, 1);

    let cards: Vec<&str> = out["board"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["card_id"].as_str().unwrap())
        .collect();
    assert_eq!(cards, [FILLER, STRONG, WEAK]);
    assert_eq!(out["board"][1]["atk"], 20);
    let targets: Vec<Option<i64>> = out["kit"]["powers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p.get("target").and_then(Value::as_i64))
        .collect();
    // FILLER stays first, WEAK third; the power naming the minion bought
    // names none.
    assert_eq!(targets, [Some(1), Some(3), None]);
}

#[test]
fn the_alternative_skipped_is_serialized_by_card_id() {
    let g = sale_then(&["buy"], &[WEAK], vec![minion(WEAK, 1, 1)]);

    let r = run(&g);

    let v = serde_json::to_value(&rebuilt(&r, 0).alternatives[0]).unwrap();
    assert_eq!(v["instead_of"], json!(WEAK));
    assert_eq!(
        v["approx"],
        json!(["earlier_stats", "battlecry_not_played"])
    );
    let g = sale_then(&["roll"], &[], vec![minion(WEAK, 1, 1)]);
    let v = serde_json::to_value(&rebuilt(&run(&g), 0).alternatives[0]).unwrap();
    assert_eq!(v["instead_of"], Value::Null);
    assert_eq!(v["approx"], json!(["earlier_stats", "gold_maybe_short"]));
}
