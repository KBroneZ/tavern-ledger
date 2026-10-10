//! The shop turn by turn (T-204) and logged actions per minute (T-205), read
//! from a saved report, for the recap and the stats. The window only renders.
//!
//! Unknown is not zero: a game saved before the parser kept the shop says
//! "not recorded", a turn the log skipped has no numbers, and an APM needs a
//! time span the log gives. The gold and pass numbers of parser revision 4
//! (T-210) are None in turns saved before it.

use serde::Serialize;
use serde_json::Value;

use bg_parser::shop::{ActionKind, ShopRecord, ShopTurn};

use crate::provenance::Source;

/// Shown next to every APM, in the recap and the stats.
pub const APM_DEFINITION: &str = "Logged actions per minute: each option and choice the game's \
    log records you sending (buys, sells, rolls, freezes, tier-ups, cards and hero powers played, \
    minions moved, discover picks), divided by the minutes from the start of the first shop to \
    the end of the game. For a turn: its actions over the time from its shop's start to the next \
    shop's start. Only what the log records: no keyboard or mouse input is read.";

const MS_PER_MINUTE: f64 = 60_000.0;
/// The parser never writes more shop turns than this.
const MAX_TURN: i64 = 100;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ShopState {
    Recorded,
    /// Saved by a parser older than revision 3: re-read with `--reparse`.
    NotRecorded,
    /// The game could not be read, or its saved shop record is malformed.
    NotAvailable,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OfferView {
    pub card_id: Option<String>,
    /// Kept from the last turn by a freeze.
    pub frozen: bool,
}

/// One shop the player saw: the turn's first (roll 0) or the one after a roll.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ShopView {
    pub roll: u32,
    pub offers: Vec<OfferView>,
}

/// One turn. `in_log` false is a turn the log skipped: every value is None.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TurnView {
    pub turn: i64,
    pub in_log: bool,
    pub tier: Option<i64>,
    pub tier_up: bool,
    pub gold: Option<i64>,
    pub gold_spent: Option<i64>,
    pub rolls: Option<u32>,
    pub free_rolls: Option<u32>,
    pub buys: Vec<Option<String>>,
    pub spell_buys: Option<u32>,
    pub sells: Vec<Option<String>>,
    pub freezes: Option<u32>,
    pub shops: Vec<ShopView>,
    pub actions: Option<u32>,
    pub seconds: Option<f64>,
    pub apm: Option<f64>,
    /// Gold beyond the turn's own and beyond sells (cards, trinkets, refunds).
    pub extra_gold: Option<u32>,
    pub sell_gold: Option<u32>,
    pub buy_gold: Option<u32>,
    pub spell_gold: Option<u32>,
    /// Rolls that used a free roll the game showed on the roll button.
    pub free_refreshes: Option<u32>,
    /// Cards passed to the teammate (Duos); None when not recorded.
    pub passes: Option<Vec<Option<String>>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TierUpView {
    pub turn: i64,
    pub tier: i64,
}

/// Sums over the turns the log has.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct ShopTotals {
    pub rolls: u32,
    pub free_rolls: u32,
    pub buys: u32,
    pub spell_buys: u32,
    pub sells: u32,
    pub freezes: u32,
    /// None when a turn's gold is not in the log.
    pub gold_spent: Option<i64>,
    pub actions: u32,
    // Parser revision 4 (T-210): None when a turn lacks the number (not
    // recorded, or its gold is not in the log).
    pub extra_gold: Option<i64>,
    pub sell_gold: Option<i64>,
    pub buy_gold: Option<i64>,
    pub spell_gold: Option<i64>,
    /// Gold beyond the fixed income: extra gold plus sell gold.
    pub gold_beyond_income: Option<i64>,
    pub free_refreshes: Option<u32>,
    pub passes: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ShopRecap {
    pub state: ShopState,
    pub source: Source,
    pub turns: Vec<TurnView>,
    pub tier_ups: Vec<TierUpView>,
    pub totals: Option<ShopTotals>,
    /// Actions by kind, most first.
    pub action_kinds: Vec<(ActionKind, u32)>,
    pub minutes: Option<f64>,
    pub apm: Option<f64>,
    /// False when the log stops before the game ends.
    pub ended: Option<bool>,
    pub apm_definition: &'static str,
}

/// The shop record of a saved report, or why there is none.
pub fn record_of(report: &Value) -> Result<ShopRecord, ShopState> {
    match report.get("shop") {
        None => Err(ShopState::NotRecorded),
        Some(Value::Null) => Err(ShopState::NotAvailable),
        Some(shop) => serde_json::from_value(shop.clone()).map_err(|_| ShopState::NotAvailable),
    }
}

/// A game whose shop the log has but whose actions it does not (no
/// `SendOption` line at all) has no APM: unknown, not zero.
fn actions_logged(record: &ShopRecord) -> bool {
    !record.actions.is_empty()
}

fn per_minute(count: usize, from_ms: i64, to_ms: i64) -> Option<f64> {
    let minutes = (to_ms - from_ms) as f64 / MS_PER_MINUTE;
    (minutes > 0.0).then(|| count as f64 / minutes)
}

/// Logged actions per minute of the game, when the log gives its span.
pub fn game_apm(record: &ShopRecord) -> Option<f64> {
    if !actions_logged(record) {
        return None;
    }
    per_minute(record.actions.len(), record.start_ms?, record.end_ms?)
}

fn turn_view(t: &ShopTurn, tier_up: bool, logged: bool) -> TurnView {
    let mut shops: Vec<ShopView> = Vec::new();
    for offer in &t.offers {
        let view = OfferView {
            card_id: offer.card_id.clone(),
            frozen: offer.frozen,
        };
        match shops.last_mut() {
            Some(shop) if shop.roll == offer.roll => shop.offers.push(view),
            _ => shops.push(ShopView {
                roll: offer.roll,
                offers: vec![view],
            }),
        }
    }
    let seconds = (t.end_ms > t.start_ms).then(|| (t.end_ms - t.start_ms) as f64 / 1000.0);
    TurnView {
        turn: t.turn,
        in_log: true,
        tier: t.tier,
        tier_up,
        gold: t.gold,
        gold_spent: t.gold_spent,
        rolls: Some(t.rolls),
        free_rolls: Some(t.free_rolls),
        buys: t.buys.clone(),
        spell_buys: Some(t.spell_buys),
        sells: t.sells.clone(),
        freezes: Some(t.freezes),
        shops,
        actions: logged.then_some(t.actions),
        seconds,
        apm: logged
            .then(|| per_minute(t.actions as usize, t.start_ms, t.end_ms))
            .flatten(),
        extra_gold: t.extra_gold,
        sell_gold: t.sell_gold,
        buy_gold: t.buy_gold,
        spell_gold: t.spell_gold,
        free_refreshes: t.free_refreshes,
        passes: t.passes.clone(),
    }
}

fn gap(turn: i64) -> TurnView {
    TurnView {
        turn,
        in_log: false,
        tier: None,
        tier_up: false,
        gold: None,
        gold_spent: None,
        rolls: None,
        free_rolls: None,
        buys: Vec::new(),
        spell_buys: None,
        sells: Vec::new(),
        freezes: None,
        shops: Vec::new(),
        actions: None,
        seconds: None,
        apm: None,
        extra_gold: None,
        sell_gold: None,
        buy_gold: None,
        spell_gold: None,
        free_refreshes: None,
        passes: None,
    }
}

/// The sum of a number over every turn, None when any turn lacks it.
fn sum_known(record: &ShopRecord, f: fn(&ShopTurn) -> Option<u32>) -> Option<i64> {
    record.turns.iter().map(|t| f(t).map(i64::from)).sum()
}

pub fn totals(record: &ShopRecord) -> ShopTotals {
    let sum = |f: &dyn Fn(&ShopTurn) -> u32| record.turns.iter().map(f).sum();
    let extra_gold = sum_known(record, |t| t.extra_gold);
    let sell_gold = sum_known(record, |t| t.sell_gold);
    ShopTotals {
        rolls: sum(&|t| t.rolls),
        free_rolls: sum(&|t| t.free_rolls),
        buys: sum(&|t| t.buys.len() as u32),
        spell_buys: sum(&|t| t.spell_buys),
        sells: sum(&|t| t.sells.len() as u32),
        freezes: sum(&|t| t.freezes),
        gold_spent: record.turns.iter().map(|t| t.gold_spent).sum(),
        actions: record.actions.len() as u32,
        extra_gold,
        sell_gold,
        buy_gold: sum_known(record, |t| t.buy_gold),
        spell_gold: sum_known(record, |t| t.spell_gold),
        gold_beyond_income: extra_gold.zip(sell_gold).map(|(e, s)| e + s),
        free_refreshes: sum_known(record, |t| t.free_refreshes).map(|n| n as u32),
        passes: record
            .turns
            .iter()
            .map(|t| t.passes.as_ref().map(|p| p.len() as u32))
            .sum(),
    }
}

fn kinds(record: &ShopRecord) -> Vec<(ActionKind, u32)> {
    let mut counts: Vec<(ActionKind, u32)> = Vec::new();
    for action in &record.actions {
        match counts.iter_mut().find(|(k, _)| *k == action.kind) {
            Some(slot) => slot.1 += 1,
            None => counts.push((action.kind, 1)),
        }
    }
    counts.sort_by_key(|(_, n)| std::cmp::Reverse(*n)); // stable: first seen on ties
    counts
}

/// The shop part of a game's recap.
pub fn shop_recap(report: &Value) -> ShopRecap {
    let record = match record_of(report) {
        Ok(record) => record,
        Err(state) => {
            return ShopRecap {
                state,
                source: Source::Unknown,
                turns: Vec::new(),
                tier_ups: Vec::new(),
                totals: None,
                action_kinds: Vec::new(),
                minutes: None,
                apm: None,
                ended: None,
                apm_definition: APM_DEFINITION,
            }
        }
    };
    let logged = actions_logged(&record);
    let mut turns: Vec<TurnView> = Vec::new();
    for t in &record.turns {
        // Turns the log skipped show as gaps, not as zeros (a hand-edited
        // turn number cannot make the list grow without end).
        let next = turns.last().map_or(1, |v| v.turn + 1);
        turns.extend((next..t.turn.min(MAX_TURN)).map(gap));
        let tier_up = record.tier_ups.iter().any(|u| u.turn == t.turn);
        turns.push(turn_view(t, tier_up, logged));
    }
    let has_turns = !record.turns.is_empty();
    let minutes = match (record.start_ms, record.end_ms) {
        (Some(start), Some(end)) if end > start => Some((end - start) as f64 / MS_PER_MINUTE),
        _ => None,
    };
    ShopRecap {
        state: ShopState::Recorded,
        source: Source::Log,
        tier_ups: record
            .tier_ups
            .iter()
            .map(|u| TierUpView {
                turn: u.turn,
                tier: u.tier,
            })
            .collect(),
        totals: has_turns.then(|| totals(&record)),
        action_kinds: kinds(&record),
        minutes,
        apm: game_apm(&record),
        ended: Some(record.ended),
        apm_definition: APM_DEFINITION,
        turns,
    }
}

/// Average turn of reaching a tier, over the games that reached it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TierTurn {
    pub tier: i64,
    pub average_turn: f64,
    pub games: usize,
}

/// Averages per game over the games whose shop the log has from start to end.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct ShopStats {
    /// Games counted: shop recorded and the game's end in the log.
    pub games: usize,
    /// Saved by an older parser (re-read with `--reparse`).
    pub not_recorded: usize,
    /// The log stops before the game ends, or the shop could not be read.
    pub not_counted: usize,
    pub rolls: Option<f64>,
    pub free_rolls: Option<f64>,
    pub buys: Option<f64>,
    pub sells: Option<f64>,
    pub gold_spent: Option<f64>,
    /// Games with every turn's gold in the log: the denominator of `gold_spent`.
    pub gold_games: usize,
    pub apm: Option<f64>,
    /// Games with a time span in the log: the denominator of `apm`.
    pub apm_games: usize,
    /// Per game: gold beyond the turn's own and beyond sells, gold from
    /// sells, both together, free rolls used, cards passed (T-210).
    pub extra_gold: Option<f64>,
    pub sell_gold: Option<f64>,
    pub gold_beyond_income: Option<f64>,
    pub free_refreshes: Option<f64>,
    pub passes: Option<f64>,
    /// Games with the gold and pass numbers for every turn: the denominator
    /// of all but `free_refreshes` (games saved before parser revision 4
    /// have none).
    pub extras_games: usize,
    /// Games whose log gives the roll button's free rolls: the denominator
    /// of `free_refreshes`.
    pub free_refresh_games: usize,
    pub tier_turns: Vec<TierTurn>,
    pub source: Source,
    pub apm_definition: &'static str,
}

/// Sums for [`ShopStats`], filled game by game.
#[derive(Debug, Default)]
pub struct ShopCounts {
    stats: ShopStats,
    rolls: u64,
    free_rolls: u64,
    buys: u64,
    sells: u64,
    gold: i64,
    apm: f64,
    extra_gold: i64,
    sell_gold: i64,
    free_refreshes: u64,
    passes: u64,
    tiers: std::collections::BTreeMap<i64, (i64, usize)>,
}

impl ShopCounts {
    pub fn add(&mut self, report: &Value) {
        let record = match record_of(report) {
            // A game with no shop turn in the log has nothing to average.
            Ok(record) if record.ended && !record.turns.is_empty() => record,
            Ok(_) | Err(ShopState::NotAvailable) => {
                // An unreadable game has nothing to count anyway; it is not
                // "not recorded".
                if report.get("status").and_then(Value::as_str) != Some("unsupported") {
                    self.stats.not_counted += 1;
                }
                return;
            }
            Err(_) => {
                self.stats.not_recorded += 1;
                return;
            }
        };
        let t = totals(&record);
        self.stats.games += 1;
        self.rolls += u64::from(t.rolls);
        self.free_rolls += u64::from(t.free_rolls);
        self.buys += u64::from(t.buys);
        self.sells += u64::from(t.sells);
        if let Some(gold) = t.gold_spent {
            self.gold = self.gold.saturating_add(gold);
            self.stats.gold_games += 1;
        }
        if let Some(apm) = game_apm(&record) {
            self.apm += apm;
            self.stats.apm_games += 1;
        }
        if let (Some(extra), Some(sell), Some(passes)) = (t.extra_gold, t.sell_gold, t.passes) {
            self.extra_gold = self.extra_gold.saturating_add(extra);
            self.sell_gold = self.sell_gold.saturating_add(sell);
            self.passes += u64::from(passes);
            self.stats.extras_games += 1;
        }
        if let Some(free) = t.free_refreshes {
            self.free_refreshes += u64::from(free);
            self.stats.free_refresh_games += 1;
        }
        // The first time each tier was reached in this game.
        let mut seen = std::collections::BTreeSet::new();
        for up in &record.tier_ups {
            if seen.insert(up.tier) {
                let slot = self.tiers.entry(up.tier).or_default();
                slot.0 = slot.0.saturating_add(up.turn);
                slot.1 += 1;
            }
        }
    }

    pub fn finish(self) -> ShopStats {
        let games = self.stats.games;
        let avg = |sum: f64, n: usize| (n > 0).then(|| sum / n as f64);
        ShopStats {
            rolls: avg(self.rolls as f64, games),
            free_rolls: avg(self.free_rolls as f64, games),
            buys: avg(self.buys as f64, games),
            sells: avg(self.sells as f64, games),
            gold_spent: avg(self.gold as f64, self.stats.gold_games),
            apm: avg(self.apm, self.stats.apm_games),
            extra_gold: avg(self.extra_gold as f64, self.stats.extras_games),
            sell_gold: avg(self.sell_gold as f64, self.stats.extras_games),
            gold_beyond_income: avg(
                self.extra_gold.saturating_add(self.sell_gold) as f64,
                self.stats.extras_games,
            ),
            free_refreshes: avg(self.free_refreshes as f64, self.stats.free_refresh_games),
            passes: avg(self.passes as f64, self.stats.extras_games),
            tier_turns: self
                .tiers
                .into_iter()
                .map(|(tier, (sum, n))| TierTurn {
                    tier,
                    average_turn: sum as f64 / n as f64,
                    games: n,
                })
                .collect(),
            source: Source::log_if(games > 0),
            apm_definition: APM_DEFINITION,
            ..self.stats
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn turn(n: i64, start: i64, end: i64, actions: u32) -> Value {
        json!({
            "turn": n, "start_ms": start, "end_ms": end, "tier": 1, "gold": 3, "gold_spent": 3,
            "rolls": 2, "free_rolls": 1, "buys": ["BG_A"], "spell_buys": 0, "sells": [],
            "freezes": 0, "actions": actions,
            "offers": [
                {"card_id": "BG_A", "roll": 0, "frozen": true},
                {"card_id": "BG_B", "roll": 0, "frozen": false},
                {"card_id": "BG_C", "roll": 2, "frozen": false},
            ],
        })
    }

    fn report(turns: Vec<Value>, actions: usize, ended: bool) -> Value {
        let action = json!({"ms": 1000, "turn": 1, "kind": "roll"});
        json!({
            "status": "ok",
            "shop": {
                "turns": turns, "tier_ups": [{"turn": 3, "tier": 2}],
                "actions": vec![action; actions],
                "start_ms": 0, "end_ms": 120_000, "ended": ended,
            },
        })
    }

    #[test]
    fn apm_is_actions_over_the_minutes_from_the_first_shop_to_the_end() {
        let r = shop_recap(&report(vec![turn(1, 0, 60_000, 6)], 24, true));
        assert_eq!(r.state, ShopState::Recorded);
        assert_eq!(r.minutes, Some(2.0));
        assert_eq!(r.apm, Some(12.0));
        assert_eq!(r.turns[0].apm, Some(6.0));
        assert_eq!(r.turns[0].seconds, Some(60.0));
        assert_eq!(r.ended, Some(true));
    }

    #[test]
    fn a_game_with_no_time_span_has_no_apm_not_zero() {
        let mut rep = report(vec![turn(1, 0, 0, 6)], 6, true);
        rep["shop"]["end_ms"] = json!(0);
        let r = shop_recap(&rep);
        assert_eq!((r.apm, r.minutes, r.turns[0].apm), (None, None, None));
    }

    #[test]
    fn offers_are_grouped_by_shop_with_the_frozen_ones_marked() {
        let r = shop_recap(&report(vec![turn(1, 0, 60_000, 6)], 6, true));
        let shops = &r.turns[0].shops;
        assert_eq!(shops.len(), 2);
        assert_eq!((shops[0].roll, shops[0].offers.len()), (0, 2));
        assert!(shops[0].offers[0].frozen && !shops[0].offers[1].frozen);
        assert_eq!(shops[1].roll, 2);
    }

    #[test]
    fn a_turn_the_log_skipped_has_no_data_not_zero() {
        let r = shop_recap(&report(
            vec![turn(1, 0, 60_000, 1), turn(3, 60_000, 120_000, 1)],
            2,
            true,
        ));
        let numbers: Vec<_> = r
            .turns
            .iter()
            .map(|t| (t.turn, t.in_log, t.rolls))
            .collect();
        assert_eq!(
            numbers,
            [(1, true, Some(2)), (2, false, None), (3, true, Some(2))]
        );
        assert!(r.turns[2].tier_up);
    }

    #[test]
    fn older_and_unreadable_games_say_so() {
        let old = shop_recap(&json!({"status": "ok"}));
        assert_eq!(
            (old.state, old.source),
            (ShopState::NotRecorded, Source::Unknown)
        );
        assert!(old.totals.is_none() && old.apm.is_none());
        let none = shop_recap(&json!({"status": "unsupported", "shop": null}));
        assert_eq!(none.state, ShopState::NotAvailable);
        let broken = shop_recap(&json!({"status": "ok", "shop": {"turns": "x"}}));
        assert_eq!(broken.state, ShopState::NotAvailable);
    }

    #[test]
    fn totals_and_kinds_add_up() {
        let r = shop_recap(&report(
            vec![turn(1, 0, 60_000, 6), turn(2, 60_000, 120_000, 6)],
            3,
            true,
        ));
        let t = r.totals.expect("totals");
        assert_eq!(
            (t.rolls, t.free_rolls, t.buys, t.gold_spent, t.actions),
            (4, 2, 2, Some(6), 3)
        );
        assert_eq!(r.action_kinds, [(ActionKind::Roll, 3)]);
    }

    #[test]
    fn stats_average_games_with_their_end_in_the_log_only() {
        let mut counts = ShopCounts::default();
        counts.add(&report(vec![turn(1, 0, 60_000, 6)], 24, true));
        counts.add(&report(
            vec![turn(1, 0, 60_000, 6), turn(2, 60_000, 120_000, 0)],
            12,
            true,
        ));
        counts.add(&report(vec![turn(1, 0, 60_000, 6)], 24, false)); // cut short
        counts.add(&json!({"status": "ok"})); // older parser
        counts.add(&json!({"status": "unsupported", "shop": null}));
        let s = counts.finish();
        assert_eq!((s.games, s.not_counted, s.not_recorded), (2, 1, 1));
        assert_eq!(s.rolls, Some(3.0));
        assert_eq!(s.gold_spent, Some(4.5));
        assert_eq!(s.apm, Some(9.0));
        assert_eq!(
            s.tier_turns,
            [TierTurn {
                tier: 2,
                average_turn: 3.0,
                games: 2
            }]
        );
        assert_eq!(s.source, Source::Log);
    }

    #[test]
    fn a_game_without_shop_turns_or_logged_actions_is_not_zeros() {
        let mut empty = report(Vec::new(), 0, true);
        empty["shop"]["start_ms"] = json!(null);
        let r = shop_recap(&empty);
        assert_eq!((r.totals.clone(), r.apm), (None, None));
        let mut counts = ShopCounts::default();
        counts.add(&empty);
        let s = counts.finish();
        assert_eq!((s.games, s.not_counted, s.gold_games), (0, 1, 0));
        // Shop turns but no option line in the log: APM unknown.
        let r = shop_recap(&report(vec![turn(1, 0, 60_000, 0)], 0, true));
        assert_eq!(
            (r.apm, r.turns[0].apm, r.turns[0].actions),
            (None, None, None)
        );
        let mut counts = ShopCounts::default();
        counts.add(&report(vec![turn(1, 0, 60_000, 0)], 0, true));
        assert_eq!(counts.finish().apm_games, 0);
    }

    #[test]
    fn a_hand_edited_turn_number_does_not_grow_the_gaps() {
        let r = shop_recap(&report(vec![turn(1_000_000_000, 0, 60_000, 1)], 1, true));
        assert!(r.turns.len() <= 101);
    }

    /// A turn as parser revision 4 writes it.
    fn turn4(n: i64, extra: u32, sell: u32, passes: &[&str]) -> Value {
        let mut t = turn(n, (n - 1) * 60_000, n * 60_000, 1);
        t["extra_gold"] = json!(extra);
        t["sell_gold"] = json!(sell);
        t["buy_gold"] = json!(3);
        t["spell_gold"] = json!(0);
        t["free_refreshes"] = json!(1);
        t["passes"] = json!(passes);
        t
    }

    #[test]
    fn revision_4_turns_show_and_total_their_gold_and_passes() {
        let r = shop_recap(&report(
            vec![turn4(1, 2, 1, &["BG_P"]), turn4(2, 0, 3, &[])],
            2,
            true,
        ));
        assert_eq!(r.turns[0].extra_gold, Some(2));
        assert_eq!(r.turns[0].passes, Some(vec![Some("BG_P".to_string())]));
        let t = r.totals.unwrap();
        assert_eq!(
            (t.extra_gold, t.sell_gold, t.gold_beyond_income),
            (Some(2), Some(4), Some(6))
        );
        assert_eq!((t.buy_gold, t.spell_gold), (Some(6), Some(0)));
        assert_eq!((t.free_refreshes, t.passes), (Some(2), Some(1)));
    }

    #[test]
    fn older_turns_have_no_revision_4_numbers_not_zeros() {
        // Revision 3 turns: the keys are not there.
        let r = shop_recap(&report(vec![turn(1, 0, 60_000, 1)], 1, true));
        assert_eq!(
            (r.turns[0].extra_gold, r.turns[0].passes.clone()),
            (None, None)
        );
        let t = r.totals.unwrap();
        assert_eq!(
            (t.extra_gold, t.gold_beyond_income, t.passes),
            (None, None, None)
        );
        // One turn without its gold in the log: the game's gold totals are unknown.
        let mut cut = turn4(2, 1, 1, &[]);
        cut["extra_gold"] = Value::Null;
        let r = shop_recap(&report(vec![turn4(1, 2, 1, &[]), cut], 2, true));
        let t = r.totals.unwrap();
        assert_eq!(
            (t.extra_gold, t.sell_gold, t.gold_beyond_income),
            (None, Some(2), None)
        );
    }

    #[test]
    fn stats_average_the_revision_4_numbers_over_the_games_that_have_them() {
        let mut counts = ShopCounts::default();
        counts.add(&report(vec![turn4(1, 2, 1, &["BG_P"])], 1, true));
        counts.add(&report(vec![turn4(1, 4, 3, &[])], 1, true));
        counts.add(&report(vec![turn(1, 0, 60_000, 1)], 1, true)); // revision 3
        let s = counts.finish();
        assert_eq!((s.games, s.extras_games), (3, 2));
        assert_eq!((s.extra_gold, s.sell_gold), (Some(3.0), Some(2.0)));
        assert_eq!(s.gold_beyond_income, Some(5.0));
        assert_eq!((s.free_refreshes, s.passes), (Some(1.0), Some(0.5)));
        assert_eq!(s.free_refresh_games, 2);
        // A game whose log never gives the button's free rolls still counts
        // for the gold, not for the free rolls.
        let mut counts = ShopCounts::default();
        let mut unknown = turn4(1, 2, 1, &[]);
        unknown["free_refreshes"] = Value::Null;
        counts.add(&report(vec![unknown], 1, true));
        let s = counts.finish();
        assert_eq!((s.extras_games, s.free_refresh_games), (1, 0));
        assert_eq!((s.extra_gold, s.free_refreshes), (Some(2.0), None));
        let none = ShopCounts::default().finish();
        assert_eq!((none.extra_gold, none.extras_games), (None, 0));
    }

    #[test]
    fn no_counted_game_is_unknown_not_zero() {
        let s = ShopCounts::default().finish();
        assert_eq!((s.rolls, s.apm, s.gold_spent), (None, None, None));
        assert_eq!(s.source, Source::Unknown);
        assert!(s.tier_turns.is_empty());
    }
}
