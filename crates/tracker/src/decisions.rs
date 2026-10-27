//! The player's shop decisions, one by one, with what was on offer (T-656),
//! from a saved report: each roll, buy, sell, freeze, tier-up and choice
//! (discovers, trinkets, Dark Gifts, quests) the log records, in order, for
//! the counterfactual engine and the game review (Phase 4).
//!
//! What the log fully shows is kept as it is: a buy is the shop the player
//! had when buying (the offers of that roll, less what was already bought
//! from it), a choice its options and the pick. What had to be matched up is
//! marked `inferred`, for every decision of that kind in the turn when the
//! counts do not agree:
//! - rolls, buys and freezes when the turn's roll actions are not its rolls,
//!   an offer has a roll the turn did not reach, or one roll shows more
//!   minions than a shop has (a card or hero power that refreshed the shop
//!   does not move the roll count, so its offers join the roll before);
//! - buys or sells when their actions and the recorded cards disagree, and a
//!   buy of a card that shop did not show;
//! - tier-ups when their actions and the recorded tier-ups disagree (a card
//!   can raise the tier too);
//! - choices when their actions and the recorded choices disagree, and a
//!   choice without a pick.
//!
//! A decision the record has but no action matched comes last in its turn,
//! without a time. A sell names the minion sold; the board it was sold from
//! is not in the shop record. A choice keeps its first pick. A record larger
//! than the parser ever writes, or with unreadable choices, is not read.
//!
//! Only the player's own shop: the log has no one else's (D-004).

use serde::Serialize;
use serde_json::Value;

use bg_parser::discover::{Discover, MAX_DISCOVERS};
use bg_parser::shop::{
    ActionKind, Offer, PlayerAction, ShopRecord, ShopTurn, TierUp, MAX_ACTIONS, MAX_GAME_OFFERS,
    MAX_TURNS,
};

use crate::shop::{record_of, ShopState};

/// The most minions one shop shows (tier 6, plus one from a card).
const MAX_SHOP: usize = 7;

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
    /// What there was to choose from: the shop the player had (for a roll,
    /// the shop it rolled away; for a freeze, the shop it kept), or the
    /// options of a choice.
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
    let discovers: Vec<Discover> = match report.get("discovers") {
        None | Some(Value::Null) => Vec::new(),
        Some(d) => serde_json::from_value(d.clone()).map_err(|_| ShopState::NotAvailable)?,
    };
    let offers: usize = record.turns.iter().map(|t| t.offers.len()).sum();
    if record.turns.len() > MAX_TURNS
        || offers > MAX_GAME_OFFERS
        || record.actions.len() > MAX_ACTIONS
        || discovers.len() > MAX_DISCOVERS
    {
        return Err(ShopState::NotAvailable);
    }
    Ok(record
        .turns
        .iter()
        .flat_map(|t| Turn::new(t, &record, &discovers).decisions())
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
        self.roll = self.roll.saturating_add(1);
        self.bought.clear();
    }
}

/// One shop turn: its actions in order, its records, and which counts agree.
struct Turn<'a> {
    t: &'a ShopTurn,
    actions: Vec<&'a PlayerAction>,
    tier_ups: Vec<&'a TierUp>,
    choices: Vec<&'a Discover>,
    shop_agrees: bool,
    buys_agree: bool,
    sells_agree: bool,
    tiers_agree: bool,
    choices_agree: bool,
}

impl<'a> Turn<'a> {
    fn new(t: &'a ShopTurn, record: &'a ShopRecord, discovers: &'a [Discover]) -> Self {
        let mut actions: Vec<&PlayerAction> =
            record.actions.iter().filter(|a| a.turn == t.turn).collect();
        actions.sort_by_key(|a| a.ms);
        let count = |k: ActionKind| actions.iter().filter(|a| a.kind == k).count();
        let tier_ups: Vec<&TierUp> = record
            .tier_ups
            .iter()
            .filter(|u| u.turn == t.turn)
            .collect();
        let choices: Vec<&Discover> = discovers.iter().filter(|d| d.turn == t.turn).collect();
        let rolls = count(ActionKind::Roll);
        let per_roll_fits = t
            .offers
            .iter()
            .all(|o| t.offers.iter().filter(|x| x.roll == o.roll).count() <= MAX_SHOP);
        Turn {
            shop_agrees: u32::try_from(rolls) == Ok(t.rolls)
                && t.offers.iter().all(|o| o.roll <= t.rolls)
                && per_roll_fits,
            buys_agree: count(ActionKind::Buy) == t.buys.len(),
            sells_agree: count(ActionKind::Sell) == t.sells.len(),
            tiers_agree: count(ActionKind::TierUp) == tier_ups.len(),
            choices_agree: count(ActionKind::Choose) == choices.len(),
            t,
            actions,
            tier_ups,
            choices,
        }
    }

    fn decisions(&self) -> Vec<Decision> {
        let turn = self.t.turn;
        let mut shop = Shop {
            offers: &self.t.offers,
            roll: 0,
            bought: Vec::new(),
        };
        let (mut buys, mut sells) = (self.t.buys.iter(), self.t.sells.iter());
        let mut tiers = self.tier_ups.iter();
        let mut choices = self.choices.iter();
        let mut out = Vec::new();
        for a in &self.actions {
            let mut d = Decision::new(turn, Some(a.ms), DecisionKind::Roll);
            match a.kind {
                ActionKind::Roll => {
                    d.offered = shop.visible();
                    d.inferred = !self.shop_agrees;
                    shop.roll();
                }
                ActionKind::Buy => {
                    d.kind = DecisionKind::Buy;
                    d.offered = shop.visible();
                    d.inferred = true;
                    if let Some(card) = buys.next() {
                        let shown = d.offered.contains(card);
                        d.inferred = !(self.shop_agrees && self.buys_agree && shown);
                        d.taken = card.clone();
                        if shown {
                            shop.bought.push(card.clone());
                        }
                    }
                }
                ActionKind::BuySpell => d.kind = DecisionKind::BuySpell,
                ActionKind::Sell => {
                    d.kind = DecisionKind::Sell;
                    d.taken = sells.next().cloned().flatten();
                    d.inferred = !self.sells_agree;
                }
                ActionKind::Freeze | ActionKind::Unfreeze => {
                    d.kind = if a.kind == ActionKind::Freeze {
                        DecisionKind::Freeze
                    } else {
                        DecisionKind::Unfreeze
                    };
                    d.offered = shop.visible();
                    d.inferred = !self.shop_agrees;
                }
                ActionKind::TierUp => {
                    d.kind = DecisionKind::TierUp;
                    d.tier = tiers.next().map(|u| u.tier);
                    d.inferred = !self.tiers_agree || d.tier.is_none();
                }
                ActionKind::Choose => {
                    d = match choices.next() {
                        Some(c) => choice(turn, Some(a.ms), c),
                        None => Decision::new(turn, Some(a.ms), DecisionKind::Choose),
                    };
                    d.inferred |= !self.choices_agree;
                }
                _ => continue,
            }
            out.push(d);
        }
        // What the record has but no action matched: kept, without a time.
        for card in buys {
            let mut d = Decision::new(turn, None, DecisionKind::Buy);
            d.taken = card.clone();
            d.inferred = true;
            out.push(d);
        }
        for card in sells {
            let mut d = Decision::new(turn, None, DecisionKind::Sell);
            d.taken = card.clone();
            d.inferred = true;
            out.push(d);
        }
        for c in choices {
            let mut d = choice(turn, None, c);
            d.inferred = true;
            out.push(d);
        }
        out
    }
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
