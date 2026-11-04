//! A saved game read for the counterfactual engine: the boards of the
//! combat after a decision and of the one before, the alternative boards
//! built from them, and the engine's teams.

use combat_sim::saved::{self, CardFacts};
use combat_sim::{Keywords, Team, Warband};
use expected_place::model::{expected, Model};
use serde_json::{json, Value};

use super::sale::{after_sale, kept_instead, After};
use super::sim::{Compared, Pair, Sims};
use super::{delta, fight, Alternative, Approx, Gap, NotRebuilt, Place, Rebuilt};
use crate::decisions::{Decision, DecisionKind};
use crate::odds::Facts;

/// The most minions on a board.
const MAX_BOARD: usize = 7;

/// A saved minion's card id.
fn card_of(m: &Value) -> Option<&str> {
    m.get("card_id").and_then(Value::as_str)
}

fn copies(board: &[Value], card: &str) -> usize {
    board.iter().filter(|m| card_of(m) == Some(card)).count()
}

/// The board of an entry, as saved.
fn board_of(entry: &Value) -> &[Value] {
    entry
        .get("board")
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

/// The keywords the record names, as saved; Poisonous and Mega-Windfury
/// are not saved and come from the card data ([`saved::minion`]).
const SAVED_KEYWORDS: [(&str, Keywords); 6] = [
    ("taunt", Keywords::TAUNT),
    ("divine_shield", Keywords::DIVINE_SHIELD),
    ("reborn", Keywords::REBORN),
    ("venomous", Keywords::VENOMOUS),
    ("windfury", Keywords::WINDFURY),
    ("stealth", Keywords::STEALTH),
];

/// A minion of `card` as the record would save it, at its card data's
/// stats and keywords.
fn fresh_minion(card: &str, f: &Facts) -> Value {
    let keywords: Vec<&str> = SAVED_KEYWORDS
        .iter()
        .filter(|(_, k)| f.base.keywords.contains(*k))
        .map(|(name, _)| *name)
        .collect();
    json!({
        "card_id": card, "atk": f.base.attack, "health": f.base.health,
        "golden": false, "keywords": keywords,
    })
}

/// True when a saved minion's card or one of its Dark Gifts acts at the
/// start of combat.
fn minion_starts(m: &Value) -> bool {
    card_of(m).is_some_and(combat_sim::cards::has_start_of_combat)
        || m.get("gifts")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .any(combat_sim::gifts::has_start_of_combat)
}

/// True when a board or its hero's kit has a card acting at the start of
/// combat.
fn entry_starts(entry: &Value) -> bool {
    let kit = entry.get("kit");
    let kit_cards = ["powers", "trinkets"]
        .into_iter()
        .filter_map(|k| kit?.get(k)?.as_array())
        .flatten()
        .filter_map(card_of);
    board_of(entry).iter().any(minion_starts)
        || kit_cards
            .into_iter()
            .any(combat_sim::kit::has_start_of_combat)
}

/// The trinkets of an entry's kit, as saved.
fn trinkets(entry: &Value) -> &[Value] {
    entry
        .get("kit")
        .and_then(|k| k.get("trinkets"))
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

/// Moves each hero power's named minion (a 1-based board position) one
/// place right when a minion is put in at index `at` before it, so it still
/// names the same minion.
pub(super) fn shift_targets(entry: &mut Value, at: usize) {
    let powers = entry
        .get_mut("kit")
        .and_then(|k| k.get_mut("powers"))
        .and_then(Value::as_array_mut);
    for power in powers.into_iter().flatten() {
        let Some(target) = power.get_mut("target") else {
            continue;
        };
        let index = target
            .as_i64()
            .and_then(|p| usize::try_from(p - 1).ok())
            .filter(|i| *i >= at);
        if let Some(i) = index {
            *target = json!(i + 2);
        }
    }
}

/// The combat after a decision: its number and both sides' entries.
struct Combat<'a> {
    round: i64,
    own: &'a Value,
    opponent: &'a Value,
    /// The player's board at the combat before; empty before the first.
    before: &'a [Value],
}

/// An alternative's own entry and what it takes as given, or why it cannot
/// be played.
type Swapped = Result<(Value, Vec<Approx>), Gap>;

/// An alternative after its comparison: the card, what it takes as given,
/// its comparison (or why there is none).
type Played<'s> = (&'s str, Vec<Approx>, Result<(Compared, Place), Gap>);

/// A game read once for all of its decisions.
pub(super) struct Game<'a> {
    rounds: &'a [Value],
    local: Option<i64>,
    facts: &'a dyn Fn(&str) -> Option<Facts>,
    /// The model, or the marker every board gets instead of a number.
    model: Result<&'a Model, Place>,
    rows: Option<expected_place::Game>,
}

impl<'a> Game<'a> {
    pub(super) fn new(
        report: &'a Value,
        facts: &'a dyn Fn(&str) -> Option<Facts>,
        model: Result<&'a Model, Place>,
    ) -> Self {
        Game {
            rounds: report
                .get("rounds")
                .and_then(Value::as_array)
                .map_or(&[], Vec::as_slice),
            local: report.get("local_player_id").and_then(Value::as_i64),
            facts,
            rows: model
                .is_ok()
                .then(|| expected_place::encode(report).ok())
                .flatten(),
            model,
        }
    }

    /// The entry on `side` with a board, the local player's first.
    fn entry(&self, round: &'a Value, side: &str) -> Option<&'a Value> {
        let entries = round.get("entries").and_then(Value::as_array)?;
        let on_side = || {
            entries.iter().filter(move |e| {
                e.get("side").and_then(Value::as_str) == Some(side)
                    && matches!(e.get("board"), Some(Value::Array(_)))
            })
        };
        let mine = self.local.and_then(|seat| {
            on_side().find(|e| e.get("player_id").and_then(Value::as_i64) == Some(seat))
        });
        mine.or_else(|| on_side().next())
    }

    /// The combat after a decision of shop turn `turn`, with the player's
    /// board of combat `turn - 1` (none when `turn` is the game's first).
    fn combat(&self, turn: i64) -> Result<Combat<'a>, NotRebuilt> {
        let number = |r: &Value| r.get("number").and_then(Value::as_i64);
        let round = self.rounds.iter().find(|r| number(r) == Some(turn));
        let round = round.ok_or(NotRebuilt::NoNextCombat)?;
        let own = self.entry(round, "own").ok_or(NotRebuilt::NoNextCombat)?;
        let opponent = self.entry(round, "opponent");
        let opponent = opponent.ok_or(NotRebuilt::NoNextCombat)?;
        let first = !self
            .rounds
            .iter()
            .any(|r| number(r).is_some_and(|n| n < turn));
        let before = if first {
            &[][..]
        } else {
            let earlier = self.rounds.iter().find(|r| number(r) == Some(turn - 1));
            let earlier = earlier.and_then(|r| self.entry(r, "own"));
            board_of(earlier.ok_or(NotRebuilt::NoBoardBefore)?)
        };
        Ok(Combat {
            round: turn,
            own,
            opponent,
            before,
        })
    }

    /// The decision `list[at]` of the game's decisions `list`.
    pub(super) fn rebuild(
        &self,
        list: &[Decision],
        at: usize,
        sims: &mut Sims,
    ) -> Result<Rebuilt, NotRebuilt> {
        let d = &list[at];
        match d.kind {
            DecisionKind::Buy | DecisionKind::Choose => self.swap(d, sims),
            DecisionKind::Sell => self.keep(d, after_sale(list, at), sims),
            _ => Err(NotRebuilt::NeedsShopPlayout),
        }
    }

    /// Each other card on offer in place of the one taken.
    fn swap(&self, d: &Decision, sims: &mut Sims) -> Result<Rebuilt, NotRebuilt> {
        let taken = d.taken.as_deref().ok_or(NotRebuilt::NoCardTaken)?;
        let mut others: Vec<&str> = Vec::new();
        for card in d.offered.iter().flatten() {
            if card != taken && !others.contains(&card.as_str()) {
                others.push(card);
            }
        }
        if others.is_empty() {
            return Err(NotRebuilt::NoAlternative);
        }
        let unknown_offers = d.offered.iter().filter(|c| c.is_none()).count();
        let c = self.combat(d.turn)?;
        let board = board_of(c.own);
        let (now, was) = (copies(board, taken), copies(c.before, taken));
        let swapped: Vec<(&str, Swapped)> = if now == 1 && was == 0 {
            let at = board.iter().position(|m| card_of(m) == Some(taken));
            let at = at.ok_or(NotRebuilt::TakenNotOnBoard)?;
            if minion_starts(&board[at]) {
                return Err(NotRebuilt::StartOfCombat);
            }
            let mut first = vec![Approx::BaseStats, Approx::BattlecryNotPlayed];
            if d.kind == DecisionKind::Buy {
                first.push(Approx::GoldIgnored);
            }
            let noted = self.noted(&c, &first);
            others
                .into_iter()
                .map(|card| (card, self.minion_in(c.own, at, card, &noted)))
                .collect()
        } else if now == 0 && trinkets(c.own).iter().any(|t| card_of(t) == Some(taken)) {
            if combat_sim::kit::has_start_of_combat(taken) {
                return Err(NotRebuilt::StartOfCombat);
            }
            let noted = self.noted(&c, &[]);
            others
                .into_iter()
                .map(|card| (card, trinket_in(c.own, taken, card, &noted)))
                .collect()
        } else if now == 0 {
            return Err(NotRebuilt::TakenNotOnBoard);
        } else {
            return Err(NotRebuilt::Ambiguous);
        };
        self.play(&c, swapped, unknown_offers, sims)
    }

    /// The minion sold, kept: in place of the first minion bought after
    /// the sale, if any (`sale.rs`).
    fn keep(&self, d: &Decision, after: After, sims: &mut Sims) -> Result<Rebuilt, NotRebuilt> {
        let sold = d.taken.as_deref().ok_or(NotRebuilt::NoCardTaken)?;
        let c = self.combat(d.turn)?;
        let board = board_of(c.own);
        let (now, was) = (copies(board, sold), copies(c.before, sold));
        if was == 0 {
            return Err(NotRebuilt::SoldNotOnBoard);
        }
        if was != 1 || now != 0 {
            return Err(NotRebuilt::Ambiguous);
        }
        let from = c.before.iter().position(|m| card_of(m) == Some(sold));
        let from = from.ok_or(NotRebuilt::SoldNotOnBoard)?;
        if minion_starts(&c.before[from]) {
            return Err(NotRebuilt::StartOfCombat);
        }
        let first = match after {
            After::Unknown => return Err(NotRebuilt::OrderUnknown),
            After::Bought { buy, sold_again } => {
                return self.keep_instead(&c, sold, from, buy, sold_again, sims)
            }
            After::Spent => vec![Approx::EarlierStats, Approx::GoldMaybeShort],
            After::Nothing => vec![Approx::EarlierStats],
        };
        if board.len() >= MAX_BOARD {
            return Err(NotRebuilt::BoardFull);
        }
        let mut own = c.own.clone();
        let at = from.min(board.len());
        let mut minions = board.to_vec();
        minions.insert(at, c.before[from].clone());
        own["board"] = Value::Array(minions);
        shift_targets(&mut own, at);
        let noted = self.noted(&c, &first);
        self.play(&c, vec![(sold, Ok((own, noted)))], 0, sims)
    }

    /// The minion sold (at `from` on the board before) kept in place of
    /// `buy`, the first minion bought after the sale: the buy must be named
    /// as the log has it, new on the next combat's board (one copy, none
    /// before) and not sold again that turn.
    fn keep_instead(
        &self,
        c: &Combat,
        sold: &str,
        from: usize,
        buy: &Decision,
        sold_again: bool,
        sims: &mut Sims,
    ) -> Result<Rebuilt, NotRebuilt> {
        let bought = buy.taken.as_deref().ok_or(NotRebuilt::NextBuyNotOnBoard)?;
        if buy.inferred {
            return Err(NotRebuilt::NextBuyUncertain);
        }
        let board = board_of(c.own);
        let (now, was) = (copies(board, bought), copies(c.before, bought));
        if now == 0 {
            return Err(NotRebuilt::NextBuyNotOnBoard);
        }
        if now != 1 || was != 0 || sold_again {
            return Err(NotRebuilt::Ambiguous);
        }
        let at = board.iter().position(|m| card_of(m) == Some(bought));
        let at = at.ok_or(NotRebuilt::NextBuyNotOnBoard)?;
        if minion_starts(&board[at]) {
            return Err(NotRebuilt::StartOfCombat);
        }
        let own = kept_instead(c.own, &c.before[from], from, at);
        let noted = self.noted(c, &[Approx::EarlierStats, Approx::BattlecryNotPlayed]);
        let r = self.play(c, vec![(sold, Ok((own, noted)))], 0, sims)?;
        let alternatives = r
            .alternatives
            .into_iter()
            .map(|a| Alternative {
                instead_of: Some(bought.to_string()),
                ..a
            })
            .collect();
        Ok(Rebuilt { alternatives, ..r })
    }

    /// What every alternative of a combat takes as given: `first`, then a
    /// start-of-combat card on the boards.
    fn noted(&self, c: &Combat, first: &[Approx]) -> Vec<Approx> {
        let mut out = first.to_vec();
        if entry_starts(c.own) || entry_starts(c.opponent) {
            out.push(Approx::StartOfCombatOnBoard);
        }
        out
    }

    /// The own entry with `card` in place of the minion at `at`.
    fn minion_in(&self, own: &Value, at: usize, card: &str, noted: &[Approx]) -> Swapped {
        let f = (self.facts)(card).ok_or(Gap::NoCardData)?;
        if f.base.health <= 0 {
            return Err(Gap::NotAMinion);
        }
        if combat_sim::cards::has_start_of_combat(card) {
            return Err(Gap::StartOfCombat);
        }
        let mut own = own.clone();
        let mut minions = board_of(&own).to_vec();
        minions[at] = fresh_minion(card, &f);
        own["board"] = Value::Array(minions);
        Ok((own, noted.to_vec()))
    }

    /// Both teams in the engine's form, with both tavern tiers 0 (the
    /// damage is then the minions' alone). A kit the record has but that
    /// cannot be read is a bad board, not a hero without one.
    fn teams(&self, own: &Value, opponent: &Value, round: i64) -> Result<Pair, Gap> {
        let lacking = std::cell::Cell::new(false);
        let facts = |id: &str| {
            let f = (self.facts)(id);
            if f.is_none() {
                lacking.set(true);
            }
            f.map(|f| CardFacts {
                tier: f.tier,
                tribes: f.tribes,
                base: Some(f.base),
            })
        };
        let number = u32::try_from(round).ok();
        let band = |entry: &Value| {
            let b = saved::board(entry, &facts)?;
            let kit = saved::kit(entry, number, &facts);
            if entry.get("kit").is_some() && kit.is_none() {
                return None;
            }
            Some(Warband {
                kit,
                tavern_tier: Some(0),
                ..b
            })
        };
        let (own, opponent) = (band(own), band(opponent));
        if lacking.get() {
            return Err(Gap::NoCardData);
        }
        match (own, opponent) {
            (Some(o), Some(p)) => Ok((Team::solo(o), Team::solo(p))),
            _ => Err(Gap::BadBoard),
        }
    }

    /// The place the model expects entering combat `round` with `board`.
    /// The board's Deathrattle count is unknown on every board played, as
    /// the card data does not give it for a card put in.
    fn place(&self, round: i64, board: &[Value]) -> Place {
        let model = match self.model {
            Ok(m) => m,
            Err(marker) => return marker,
        };
        let Some(row) = self
            .rows
            .as_ref()
            .and_then(|g| g.turns.iter().find(|t| t.turn == round))
        else {
            return Place::NoRow;
        };
        let mut values = row.values;
        values[expected_place::OWN_BOARD].copy_from_slice(&expected_place::board_values(board));
        if let Some(i) = expected_place::NAMES
            .iter()
            .position(|n| *n == "deathrattle")
        {
            values[i] = None;
        }
        let row = expected_place::Turn {
            turn: round,
            values,
        };
        Place::Expected(expected(&model.predict(&row)))
    }

    /// Plays one alternative against the actual board.
    fn compare<'s>(
        &self,
        c: &Combat,
        actual: &Pair,
        card: &'s str,
        alt: Swapped,
        sims: &mut Sims,
    ) -> Result<Played<'s>, NotRebuilt> {
        let (own, approx) = match alt {
            Ok(a) => a,
            Err(gap) => return Ok((card, Vec::new(), Err(gap))),
        };
        let teams = match self.teams(&own, c.opponent, c.round) {
            Ok(t) => t,
            Err(gap) => return Ok((card, approx, Err(gap))),
        };
        let compared = sims.compare(actual, &teams).map_err(NotRebuilt::Actual)?;
        let place = self.place(c.round, board_of(&own));
        Ok((card, approx, Ok((compared, place))))
    }

    /// Plays the actual board and each alternative.
    fn play(
        &self,
        c: &Combat,
        swapped: Vec<(&str, Swapped)>,
        unknown_offers: usize,
        sims: &mut Sims,
    ) -> Result<Rebuilt, NotRebuilt> {
        let actual = self
            .teams(c.own, c.opponent, c.round)
            .map_err(NotRebuilt::Actual)?;
        sims.play(&actual).map_err(NotRebuilt::Actual)?;
        let played = swapped
            .into_iter()
            .map(|(card, alt)| self.compare(c, &actual, card, alt, sims))
            .collect::<Result<Vec<_>, _>>()?;
        let gifts = saved::unknown_gifts([c.own, c.opponent]);
        let actual_place = self.place(c.round, board_of(c.own));
        let alternative = |(card, approx, result): Played| {
            let (fight, delta, stop) = match result {
                Err(gap) => (Err(gap), None, None),
                Ok((cmp, place)) => {
                    let at_stop = fight(&cmp.actual, actual_place, &gifts);
                    let alt = cmp.alternative.map(|o| (fight(&o, place, &gifts), o));
                    let d = alt
                        .as_ref()
                        .ok()
                        .map(|(f, o)| delta(&at_stop, &cmp.actual, f, o));
                    (alt.map(|(f, _)| f), d, Some(cmp.stop))
                }
            };
            Alternative {
                card: card.to_string(),
                instead_of: None,
                approx,
                fight,
                delta,
                stop,
            }
        };
        let alternatives = played.into_iter().map(alternative).collect();
        let actual_odds = sims.odds(&actual).map_err(NotRebuilt::Actual)?;
        Ok(Rebuilt {
            round: c.round,
            actual: fight(&actual_odds, actual_place, &gifts),
            alternatives,
            unknown_offers,
        })
    }
}

/// The own entry with the trinket `card` in place of `taken`.
fn trinket_in(own: &Value, taken: &str, card: &str, noted: &[Approx]) -> Swapped {
    if combat_sim::kit::has_start_of_combat(card) {
        return Err(Gap::StartOfCombat);
    }
    let mut own = own.clone();
    let slot = own
        .get_mut("kit")
        .and_then(|k| k.get_mut("trinkets"))
        .and_then(Value::as_array_mut)
        .and_then(|t| t.iter_mut().find(|t| card_of(t) == Some(taken)))
        .ok_or(Gap::BadBoard)?;
    let kept = slot.get("slot").cloned().unwrap_or(json!(1));
    *slot = json!({"card_id": card, "slot": kept});
    Ok((own, noted.to_vec()))
}
