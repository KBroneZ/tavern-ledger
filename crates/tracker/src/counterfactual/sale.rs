//! A sale weighed against what its gold paid for. Players sell to afford,
//! and make room for, what they buy next; the log does not give the gold,
//! so keeping the minion sold is not taken as free when the turn went on
//! to spend:
//! - a minion bought after the sale, in the same shop turn (so before that
//!   turn's combat): the alternative keeps the minion sold and skips the
//!   first such buy, which always costs more than the sale gave and frees
//!   the same place on the board;
//! - gold spent otherwise after it (a roll, a tier-up, a spell): the minion
//!   is kept as it is, noted as maybe short of gold
//!   ([`super::Approx::GoldMaybeShort`]);
//! - nothing that costs gold after it (a freeze is free): the minion is
//!   kept as it is.
//!
//! A turn's timed decisions are in time order ([`crate::decisions`]); one
//! without a time comes last, its place in the turn unknown.

use serde_json::{json, Value};

use super::game::shift_targets;
use crate::decisions::{Decision, DecisionKind};

/// What spends gold without buying a minion; freezing the shop is free.
const SPENT: [DecisionKind; 3] = [
    DecisionKind::Roll,
    DecisionKind::TierUp,
    DecisionKind::BuySpell,
];

/// What the player did after a sale, in the same shop turn.
#[derive(Debug, PartialEq)]
pub(super) enum After<'d> {
    /// Nothing that spends gold.
    Nothing,
    /// Gold spent, but no minion bought.
    Spent,
    /// The first minion bought after the sale; `sold_again` when a minion
    /// of its card was sold after it that turn.
    Bought { buy: &'d Decision, sold_again: bool },
    /// The sale or a buy that turn has no time: whether a buy came after
    /// the sale is not known.
    Unknown,
}

/// What came after the sale `list[at]` in its shop turn.
pub(super) fn after_sale(list: &[Decision], at: usize) -> After<'_> {
    let sale = &list[at];
    let turn = |i: usize, d: &Decision| i != at && d.turn == sale.turn;
    let timed = |d: &Decision| sale.ms.is_some() && d.ms.is_some();
    let later: Vec<(usize, &Decision)> = list
        .iter()
        .enumerate()
        .filter(|(i, d)| turn(*i, d) && (!timed(d) || *i > at))
        .collect();
    let buys = || later.iter().filter(|(_, d)| d.kind == DecisionKind::Buy);
    if buys().any(|(_, d)| !timed(d)) {
        return After::Unknown;
    }
    if let Some(&(y, buy)) = buys().next() {
        let sold_again = later.iter().any(|(i, d)| {
            d.kind == DecisionKind::Sell && (*i > y || !timed(d)) && d.taken == buy.taken
        });
        return After::Bought { buy, sold_again };
    }
    if later.iter().any(|(_, d)| SPENT.contains(&d.kind)) {
        After::Spent
    } else {
        After::Nothing
    }
}

/// The own entry with the minion at `bought_at` taken off and `kept` put
/// back at `from`, its place before the sale, with each hero power's named
/// minion moved along; a power that named the minion taken off names none.
pub(super) fn kept_instead(own: &Value, kept: &Value, from: usize, bought_at: usize) -> Value {
    let mut out = own.clone();
    let mut minions = own
        .get("board")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if bought_at < minions.len() {
        minions.remove(bought_at);
        unshift_targets(&mut out, bought_at);
    }
    let at = from.min(minions.len());
    minions.insert(at, kept.clone());
    out["board"] = Value::Array(minions);
    shift_targets(&mut out, at);
    out
}

/// Moves each hero power's named minion (a 1-based board position) one
/// place left when the minion at index `at` before it is taken off; a power
/// that named that minion names none.
fn unshift_targets(entry: &mut Value, at: usize) {
    let powers = entry
        .get_mut("kit")
        .and_then(|k| k.get_mut("powers"))
        .and_then(Value::as_array_mut);
    for power in powers.into_iter().flatten() {
        let index = power
            .get("target")
            .and_then(Value::as_i64)
            .and_then(|p| usize::try_from(p - 1).ok());
        match index {
            Some(i) if i == at => {
                if let Some(p) = power.as_object_mut() {
                    p.remove("target");
                }
            }
            Some(i) if i > at => power["target"] = json!(i),
            _ => {}
        }
    }
}
