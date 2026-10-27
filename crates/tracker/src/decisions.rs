//! The player's shop decisions, one by one, with what was on offer (T-656),
//! from a saved report: each roll, buy, sell, freeze, tier-up and choice
//! (discovers, trinkets, Dark Gifts, quests) the log records, in order, for
//! the counterfactual engine and the game review (Phase 4).
//!
//! What the log fully shows is kept as it is: a buy is the shop the player
//! had when buying (the offers of that roll, less what was already bought
//! from it), a choice its options and the pick. What had to be matched up is
//! marked `inferred`: a turn whose buy or sell actions and recorded buys or
//! sells do not agree, a buy of a card that shop did not show, a choice the
//! log answered but whose action or pick it does not have, a turn saved
//! without its actions. A sell names the minion sold; the board it was sold
//! from is not in the shop record, so it has no options.
//!
//! Only the player's own shop: the log has no one else's (D-004).

use serde::Serialize;
use serde_json::Value;

use bg_parser::discover::Discover;
use bg_parser::shop::{ActionKind, Offer, PlayerAction, ShopRecord, ShopTurn};

use crate::shop::{record_of, ShopState};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionKind {
    Roll,
    Buy,
    BuySpell,
    Sell,
    Freeze,
    Unfreeze,
    TierUp,
    /// A pick among cards offered: a discover, a trinket, a Dark Gift...
    Choose,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Decision {
    pub turn: i64,
    /// From the game's first log line; None when the log has no action for it.
    pub ms: Option<i64>,
    pub kind: DecisionKind,
    /// The card bought, sold or picked.
    pub taken: Option<String>,
    /// What there was to choose from: the shop for a roll, buy or freeze (the
    /// shop a roll leaves behind), the options of a choice.
    pub offered: Vec<Option<String>>,
    /// The card that offered a choice.
    pub source: Option<String>,
    /// The tier a tier-up reached.
    pub tier: Option<i64>,
    /// Matched up rather than read as the log has it (module docs).
    pub inferred: bool,
}

impl Decision {
    fn new(turn: i64, ms: Option<i64>, kind: DecisionKind) -> Self {
        Decision {
            turn,
            ms,
            kind,
            taken: None,
            offered: Vec::new(),
            source: None,
            tier: None,
            inferred: false,
        }
    }
}

/// Every decision of the game, turn by turn, or why the record has none.
pub fn decisions(report: &Value) -> Result<Vec<Decision>, ShopState> {
    let record = record_of(report)?;
    let discovers: Vec<Discover> = report
        .get("discovers")
        .and_then(|d| serde_json::from_value(d.clone()).ok())
        .unwrap_or_default();
    Ok(record
        .turns
        .iter()
        .flat_map(|t| turn_decisions(t, &record, &discovers))
        .collect())
}

/// The shop the player has now: the offers of the current roll, less what was
/// already bought from it.
struct Shop<'a> {
    offers: &'a [Offer],
    roll: u32,
    bought: Vec<Option<String>>,
}

impl Shop<'_> {
    fn visible(&self) -> Vec<Option<String>> {
        let mut left = self.bought.clone();
        let mut out = Vec::new();
        for o in self.offers.iter().filter(|o| o.roll == self.roll) {
            match left.iter().position(|b| *b == o.card_id) {
                Some(i) => {
                    left.remove(i);
                }
                None => out.push(o.card_id.clone()),
            }
        }
        out
    }

    fn roll(&mut self) {
        self.roll += 1;
        self.bought.clear();
    }
}

fn turn_decisions(t: &ShopTurn, record: &ShopRecord, discovers: &[Discover]) -> Vec<Decision> {
    let mut actions: Vec<&PlayerAction> =
        record.actions.iter().filter(|a| a.turn == t.turn).collect();
    actions.sort_by_key(|a| a.ms);
    let count = |k: ActionKind| actions.iter().filter(|a| a.kind == k).count();
    let buys_agree = count(ActionKind::Buy) == t.buys.len();
    let sells_agree = count(ActionKind::Sell) == t.sells.len();
    let mut tiers = record.tier_ups.iter().filter(|u| u.turn == t.turn);
    let mut choices = discovers.iter().filter(|d| d.turn == t.turn);
    let mut shop = Shop {
        offers: &t.offers,
        roll: 0,
        bought: Vec::new(),
    };
    let (mut buys, mut sells) = (t.buys.iter(), t.sells.iter());
    let mut out = Vec::new();
    for a in &actions {
        let mut d = Decision::new(t.turn, Some(a.ms), DecisionKind::Roll);
        match a.kind {
            ActionKind::Roll => {
                d.offered = shop.visible();
                shop.roll();
            }
            ActionKind::Buy => {
                let Some(card) = buys.next() else {
                    d.kind = DecisionKind::Buy;
                    d.inferred = true;
                    out.push(d);
                    continue;
                };
                d.kind = DecisionKind::Buy;
                d.offered = shop.visible();
                d.inferred = !buys_agree || !d.offered.contains(card);
                d.taken = card.clone();
                shop.bought.push(card.clone());
            }
            ActionKind::BuySpell => d.kind = DecisionKind::BuySpell,
            ActionKind::Sell => {
                d.kind = DecisionKind::Sell;
                d.taken = sells.next().cloned().flatten();
                d.inferred = !sells_agree;
            }
            ActionKind::Freeze | ActionKind::Unfreeze => {
                d.kind = if a.kind == ActionKind::Freeze {
                    DecisionKind::Freeze
                } else {
                    DecisionKind::Unfreeze
                };
                d.offered = shop.visible();
            }
            ActionKind::TierUp => {
                d.kind = DecisionKind::TierUp;
                d.tier = tiers.next().map(|u| u.tier);
            }
            ActionKind::Choose => match choices.next() {
                Some(c) => d = choice(t.turn, Some(a.ms), c),
                None => {
                    d.kind = DecisionKind::Choose;
                    d.inferred = true;
                }
            },
            _ => continue,
        }
        out.push(d);
    }
    // What the record has but no action matched: kept, without a time.
    for card in buys {
        let mut d = Decision::new(t.turn, None, DecisionKind::Buy);
        d.taken = card.clone();
        d.inferred = true;
        out.push(d);
    }
    for card in sells {
        let mut d = Decision::new(t.turn, None, DecisionKind::Sell);
        d.taken = card.clone();
        d.inferred = true;
        out.push(d);
    }
    for c in choices {
        let mut d = choice(t.turn, None, c);
        d.inferred = true;
        out.push(d);
    }
    out
}

fn choice(turn: i64, ms: Option<i64>, c: &Discover) -> Decision {
    let mut d = Decision::new(turn, ms, DecisionKind::Choose);
    d.source = c.source.clone();
    d.offered = c.offered.iter().map(|o| o.card_id.clone()).collect();
    d.taken = c
        .picked
        .first()
        .and_then(|i| c.offered.get(*i))
        .and_then(|o| o.card_id.clone());
    d.inferred = c.picked.is_empty();
    d
}

#[cfg(test)]
#[path = "decisions_tests.rs"]
mod tests;
