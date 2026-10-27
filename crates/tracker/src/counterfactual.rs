//! The counterfactual engine (T-657), for the game review (Phase 4): for
//! each decision of a saved Solo game ([`crate::decisions`]), what the
//! next combat would have looked like had the player taken another card on
//! offer, or kept the minion sold. Post-game only (D-176): it reads a saved
//! game and never a game being played.
//!
//! What is rebuilt, on the boards the record saved for the combat after
//! the decision (shop turn `t` is followed by combat `t`):
//! - a buy or a choice of a minion: the minion taken must be new on that
//!   board (one copy, none on the board of the combat before), and each
//!   other card on offer takes its place, at its card data's stats;
//! - a choice of a trinket: each other trinket on offer takes its place in
//!   the hero's kit;
//! - a sale: the minion sold (the only copy on the board of the combat
//!   before, none after) is put back where it was, with its stats then.
//!
//! Everything else says why it is not rebuilt ([`NotRebuilt`]), never
//! dropped: a roll, freeze, tier-up or spell would need shops and gold the
//! log never showed. Only the next combat is played: what an alternative
//! would have changed in later turns is not known.
//!
//! The actual board and each alternative go through the same steps: both
//! are played by the simulator (`counterfactual_sim.rs`), never compared with
//! the real result, from the boards at the first attack (the record has no
//! other), so start of combat is not played. A card that acts at the start
//! of combat cannot be swapped in or out ([`Gap::StartOfCombat`],
//! [`NotRebuilt::StartOfCombat`]); one elsewhere on the boards is noted
//! ([`Approx::StartOfCombatOnBoard`]). Both heroes' tavern tiers are left
//! out of the damage (the record lacks the opponent's, and it is the same
//! either way): the damage given is the tiers of the minions left standing.
//!
//! With a model of expected place, each board also gets the place the model
//! expects for the position entering that combat with that board
//! ([`Place`]); without one, or with one players may not see yet, a marker
//! and never a number. That place does not use the simulated fight yet:
//! the model reads both boards entering it.
//!
//! The whole game runs within a time budget ([`Budget`]); a decision the
//! time did not reach says so ([`Gap::OutOfTime`]).

use std::time::{Duration, Instant};

use combat_sim::saved::{self, CardFacts};
use combat_sim::{CombatResult, Keywords, Odds, Team, Warband};
use expected_place::model::{expected, places, Model, INPUTS};
use serde::Serialize;
use serde_json::{json, Value};

use crate::decisions::{decisions, Decision, DecisionKind};
use crate::odds::Facts;
use crate::shop::ShopState;

#[path = "counterfactual_sim.rs"]
mod counterfactual_sim;

use counterfactual_sim::{score, standard_error, Sims, Z};
pub use counterfactual_sim::{Gap, Stop};

/// The most minions on a board.
const MAX_BOARD: usize = 7;

/// How much the engine may do for one game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Budget {
    /// For the whole game, checked before every batch of trials.
    pub time: Duration,
    /// The most trials of one board.
    pub max_trials: u32,
    /// Trials per batch; whether a comparison is settled is checked
    /// between batches.
    pub batch: u32,
}

impl Default for Budget {
    /// Well within the minute the review has once a game ends.
    fn default() -> Self {
        Budget {
            time: Duration::from_secs(40),
            max_trials: 4_000,
            batch: 500,
        }
    }
}

/// Why a game gets no review.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NoReview {
    /// Not a Solo Battlegrounds game (Duos comes later).
    NotSolo,
    /// The record has no shop to read decisions from.
    NoShop(ShopState),
}

/// Why a decision is not rebuilt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NotRebuilt {
    /// A roll, freeze, unfreeze, tier-up or spell: the alternative needs
    /// shops and gold the log never showed.
    NeedsShopPlayout,
    /// The record does not say which card was taken or sold.
    NoCardTaken,
    /// Nothing else was on offer.
    NoAlternative,
    /// The combat after the decision has no board on a side: the game
    /// ended, or the combat was not shown.
    NoNextCombat,
    /// The card taken is neither on the next combat's board nor among the
    /// hero's trinkets (in hand, sold, a spell, made golden...).
    TakenNotOnBoard,
    /// The minion sold is not on the board of the combat before.
    SoldNotOnBoard,
    /// Copies of the card on the boards leave open which one it was.
    Ambiguous,
    /// Keeping the minion sold would need an eighth place on the board.
    BoardFull,
    /// The card taken or sold acts at the start of combat, which the saved
    /// boards already show or can no longer play.
    StartOfCombat,
    /// The actual board could not be played.
    Actual(Gap),
}

/// What an alternative takes as given.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Approx {
    /// The card put in has its card data's stats, without what the minion
    /// it replaces gained after it was taken.
    BaseStats,
    /// The minion kept has its stats from the combat before it was sold.
    EarlierStats,
    /// A card on the boards acts at the start of combat: it might have
    /// changed the card put in too.
    StartOfCombatOnBoard,
}

/// The place the model expects for a board, or why there is no number.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Place {
    NoModel,
    /// Trained on fewer games than players may see numbers from.
    TooFewGames,
    /// A Duos model.
    OtherMode,
    /// Trained on rows of another format, or not of the expected shape.
    OtherFormat,
    /// The record gives no row for the combat.
    NoRow,
    /// The expected place, 1 the best.
    Expected(f64),
}

impl Place {
    fn number(self) -> Option<f64> {
        match self {
            Place::Expected(p) => Some(p),
            _ => None,
        }
    }
}

/// One board's next combat as the simulator played it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Fight {
    pub won: f64,
    pub tied: f64,
    pub lost: f64,
    pub trials: u32,
    /// The tiers of the player's minions left standing, on average over
    /// every trial (0 when not won); None when a tier is unknown.
    pub minion_damage_dealt: Option<f64>,
    /// The tiers of the opponent's minions left standing, likewise.
    pub minion_damage_taken: Option<f64>,
    pub place: Place,
    /// Card ids the simulator plays with a known gap, and those of minions
    /// gifted with no gift named.
    pub approximate: Vec<String>,
}

impl Fight {
    /// A win counts 1, a tie a half.
    pub fn score(&self) -> f64 {
        self.won + 0.5 * self.tied
    }
}

/// An alternative against the actual board: each number is the
/// alternative's minus the actual one's.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Delta {
    /// The next combat's score (a win 1, a tie a half): above 0, the
    /// alternative did better in it. Not the cost in final place.
    pub score: f64,
    pub standard_error: f64,
    /// The difference is beyond three standard errors.
    pub clear: bool,
    pub minion_damage_dealt: Option<f64>,
    pub minion_damage_taken: Option<f64>,
    /// Expected place: below 0, the alternative's is better. None unless
    /// both boards have a number.
    pub place: Option<f64>,
}

/// One card the player could have taken instead, or the minion kept.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Alternative {
    pub card: String,
    pub approx: Vec<Approx>,
    pub fight: Result<Fight, Gap>,
    pub delta: Option<Delta>,
    /// Why its comparison stopped; None when it was never played.
    pub stop: Option<Stop>,
}

/// A decision played again: the actual board's next combat and each
/// alternative's.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Rebuilt {
    /// The combat played.
    pub round: i64,
    pub actual: Fight,
    pub alternatives: Vec<Alternative>,
    /// Cards on offer the log does not name: not played.
    pub unknown_offers: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Reviewed {
    pub decision: Decision,
    pub rebuilt: Result<Rebuilt, NotRebuilt>,
}

/// Every decision of a game, rebuilt or with the reason it is not.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct GameReview {
    pub decisions: Vec<Reviewed>,
    /// The time budget ran out before the end.
    pub out_of_time: bool,
    /// Boards played, each once however many decisions shared it.
    pub boards: usize,
    pub trials: u64,
}

/// Every decision of `report` (a saved game's report) played again, within
/// `budget`. `facts` gives the card data for a card id; `model` the
/// expected-place model, if any. Runs the simulator: call it off the
/// window's thread.
pub fn review(
    report: &Value,
    facts: &dyn Fn(&str) -> Option<Facts>,
    model: Option<&Model>,
    budget: &Budget,
) -> Result<GameReview, NoReview> {
    let started = Instant::now();
    review_with_clock(report, facts, model, budget, &mut || started.elapsed())
}

/// [`review`] with the time since it began given by `elapsed`.
pub fn review_with_clock(
    report: &Value,
    facts: &dyn Fn(&str) -> Option<Facts>,
    model: Option<&Model>,
    budget: &Budget,
    elapsed: &mut dyn FnMut() -> Duration,
) -> Result<GameReview, NoReview> {
    if report.get("game_type").and_then(Value::as_str) != Some("GT_BATTLEGROUNDS") {
        return Err(NoReview::NotSolo);
    }
    let list = decisions(report).map_err(NoReview::NoShop)?;
    let game = Game::new(report, facts, model);
    let mut sims = Sims::new(*budget, elapsed);
    let reviewed = list
        .into_iter()
        .map(|decision| Reviewed {
            rebuilt: game.rebuild(&decision, &mut sims),
            decision,
        })
        .collect();
    Ok(GameReview {
        decisions: reviewed,
        out_of_time: sims.out_of_time,
        boards: sims.boards(),
        trials: sims.trials,
    })
}

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

/// The combat after a decision: its number and both sides' entries.
struct Combat<'a> {
    round: i64,
    own: &'a Value,
    opponent: &'a Value,
    /// The player's board at the combat before; empty before the first.
    before: &'a [Value],
}

/// An alternative's own entry, or why it cannot be played.
type Swapped = Result<(Value, Vec<Approx>), Gap>;

/// A game read once for all of its decisions.
struct Game<'a> {
    rounds: &'a [Value],
    local: Option<i64>,
    facts: &'a dyn Fn(&str) -> Option<Facts>,
    /// The model, or the marker every board gets instead of a number.
    model: Result<&'a Model, Place>,
    rows: Option<expected_place::Game>,
}

impl<'a> Game<'a> {
    fn new(
        report: &'a Value,
        facts: &'a dyn Fn(&str) -> Option<Facts>,
        model: Option<&'a Model>,
    ) -> Self {
        let model = usable(model);
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

    fn combat(&self, turn: i64) -> Option<Combat<'a>> {
        let number = |r: &Value| r.get("number").and_then(Value::as_i64);
        let round = self.rounds.iter().find(|r| number(r) == Some(turn))?;
        let before = self
            .rounds
            .iter()
            .filter(|r| number(r).is_some_and(|n| n < turn))
            .filter_map(|r| Some((number(r)?, self.entry(r, "own")?)))
            .max_by_key(|(n, _)| *n)
            .map_or(&[][..], |(_, e)| board_of(e));
        Some(Combat {
            round: turn,
            own: self.entry(round, "own")?,
            opponent: self.entry(round, "opponent")?,
            before,
        })
    }

    fn rebuild(&self, d: &Decision, sims: &mut Sims) -> Result<Rebuilt, NotRebuilt> {
        match d.kind {
            DecisionKind::Buy | DecisionKind::Choose => self.swap(d, sims),
            DecisionKind::Sell => self.keep(d, sims),
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
        let c = self.combat(d.turn).ok_or(NotRebuilt::NoNextCombat)?;
        let board = board_of(c.own);
        let (now, was) = (copies(board, taken), copies(c.before, taken));
        let noted = self.noted(&c, Approx::BaseStats);
        let swapped: Vec<(&str, Swapped)> = if now == 1 && was == 0 {
            let at = board.iter().position(|m| card_of(m) == Some(taken));
            let at = at.ok_or(NotRebuilt::TakenNotOnBoard)?;
            if minion_starts(&board[at]) {
                return Err(NotRebuilt::StartOfCombat);
            }
            others
                .iter()
                .map(|card| (*card, self.minion_in(c.own, at, card, &noted)))
                .collect()
        } else if now == 0 && trinkets(c.own).iter().any(|t| card_of(t) == Some(taken)) {
            if combat_sim::kit::has_start_of_combat(taken) {
                return Err(NotRebuilt::StartOfCombat);
            }
            others
                .iter()
                .map(|card| (*card, trinket_in(c.own, taken, card, &noted)))
                .collect()
        } else if now == 0 {
            return Err(NotRebuilt::TakenNotOnBoard);
        } else {
            return Err(NotRebuilt::Ambiguous);
        };
        self.play(&c, swapped, unknown_offers, sims)
    }

    /// The minion sold, kept.
    fn keep(&self, d: &Decision, sims: &mut Sims) -> Result<Rebuilt, NotRebuilt> {
        let sold = d.taken.as_deref().ok_or(NotRebuilt::NoCardTaken)?;
        let c = self.combat(d.turn).ok_or(NotRebuilt::NoNextCombat)?;
        let board = board_of(c.own);
        let (now, was) = (copies(board, sold), copies(c.before, sold));
        if was == 0 {
            return Err(NotRebuilt::SoldNotOnBoard);
        }
        if was != 1 || now != 0 {
            return Err(NotRebuilt::Ambiguous);
        }
        let at = c.before.iter().position(|m| card_of(m) == Some(sold));
        let at = at.ok_or(NotRebuilt::SoldNotOnBoard)?;
        if minion_starts(&c.before[at]) {
            return Err(NotRebuilt::StartOfCombat);
        }
        if board.len() >= MAX_BOARD {
            return Err(NotRebuilt::BoardFull);
        }
        let mut own = c.own.clone();
        let mut minions = board.to_vec();
        minions.insert(at.min(minions.len()), c.before[at].clone());
        own["board"] = Value::Array(minions);
        let noted = self.noted(&c, Approx::EarlierStats);
        self.play(&c, vec![(sold, Ok((own, noted)))], 0, sims)
    }

    /// What every alternative of a combat takes as given.
    fn noted(&self, c: &Combat, first: Approx) -> Vec<Approx> {
        let mut out = vec![first];
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
    /// damage is then the minions' alone).
    fn teams(&self, own: &Value, opponent: &Value, round: i64) -> Result<(Team, Team), Gap> {
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
            Some(Warband {
                kit: saved::kit(entry, number, &facts),
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
        let mut played = Vec::new();
        for (card, alt) in swapped {
            let built = alt.and_then(|(own, approx)| {
                let teams = self.teams(&own, c.opponent, c.round)?;
                Ok((own, approx, teams))
            });
            played.push(match built {
                Err(gap) => (card, Vec::new(), Err(gap), None),
                Ok((own, approx, teams)) => {
                    let compared = sims.compare(&actual, &teams);
                    let place = self.place(c.round, board_of(&own));
                    let fight = compared.alternative.map(|o| (o, place));
                    (card, approx, fight, Some(compared.stop))
                }
            });
        }
        // Every alternative against the actual board's final trials.
        let actual_odds = sims.odds(&actual).map_err(NotRebuilt::Actual)?;
        let gifts = saved::unknown_gifts([c.own, c.opponent]);
        let actual_fight = fight(&actual_odds, self.place(c.round, board_of(c.own)), &gifts);
        let alternatives = played
            .into_iter()
            .map(|(card, approx, odds, stop)| {
                let odds = odds.map(|(o, place)| (fight(&o, place, &gifts), o));
                Alternative {
                    card: card.to_string(),
                    approx,
                    delta: odds
                        .as_ref()
                        .ok()
                        .map(|(f, o)| delta(&actual_fight, &actual_odds, f, o)),
                    fight: odds.map(|(f, _)| f),
                    stop,
                }
            })
            .collect();
        Ok(Rebuilt {
            round: c.round,
            actual: actual_fight,
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
    // A trinket swap keeps the board: its stats note does not apply.
    let approx = noted
        .iter()
        .copied()
        .filter(|a| *a != Approx::BaseStats)
        .collect();
    Ok((own, approx))
}

/// The model, or the marker every board gets instead of a number: none,
/// one players may not see yet, one of the other mode or format.
fn usable(model: Option<&Model>) -> Result<&Model, Place> {
    let m = model.ok_or(Place::NoModel)?;
    if !m.solo {
        return Err(Place::OtherMode);
    }
    let k = places(true);
    let shaped = m.encoder == expected_place::VERSION
        && m.mean.len() == expected_place::WIDTH
        && m.spread.len() == expected_place::WIDTH
        && m.weights.len() == k
        && m.weights.iter().all(|w| w.len() == INPUTS)
        && m.intercepts.len() == k;
    if !shaped {
        return Err(Place::OtherFormat);
    }
    if !m.ready() {
        return Err(Place::TooFewGames);
    }
    Ok(m)
}

/// The mean of a damage spread over every trial; None when a damage was
/// unknown.
fn mean_damage(o: &Odds, counts: &std::collections::BTreeMap<i32, u32>) -> Option<f64> {
    if o.damage_unknown > 0 || o.trials == 0 {
        return None;
    }
    let total: f64 = counts
        .iter()
        .map(|(d, n)| f64::from(*d) * f64::from(*n))
        .sum();
    Some(total / f64::from(o.trials))
}

/// A board's fight from its odds; `gifts` are the minions gifted with no
/// gift named, played as the plain card.
fn fight(o: &Odds, place: Place, gifts: &[String]) -> Fight {
    let mut approximate: Vec<String> = Vec::new();
    let ids = o.coverage.approximate.iter().map(|(id, _)| id).chain(gifts);
    for id in ids {
        if !approximate.contains(id) {
            approximate.push(id.clone());
        }
    }
    Fight {
        won: o.share(CombatResult::Won),
        tied: o.share(CombatResult::Tie),
        lost: o.share(CombatResult::Lost),
        trials: o.trials,
        minion_damage_dealt: mean_damage(o, &o.damage_dealt),
        minion_damage_taken: mean_damage(o, &o.damage_taken),
        place,
        approximate,
    }
}

fn delta(actual: &Fight, actual_odds: &Odds, alt: &Fight, alt_odds: &Odds) -> Delta {
    let d = score(alt_odds) - score(actual_odds);
    let se = standard_error(actual_odds, alt_odds);
    let minus = |a: Option<f64>, b: Option<f64>| Some(b? - a?);
    Delta {
        score: d,
        standard_error: se,
        clear: d.abs() > Z * se,
        minion_damage_dealt: minus(actual.minion_damage_dealt, alt.minion_damage_dealt),
        minion_damage_taken: minus(actual.minion_damage_taken, alt.minion_damage_taken),
        place: minus(actual.place.number(), alt.place.number()),
    }
}

#[cfg(test)]
#[path = "counterfactual_tests.rs"]
mod tests;
