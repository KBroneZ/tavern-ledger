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
pub(crate) const MAX_TURN: i64 = 100;

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

/// A number summed over the turns that have it: `turns` of the game's `of`
/// turns. A turn without it is left out, never counted as 0.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct TurnSum {
    pub value: i64,
    pub turns: u32,
    pub of: u32,
}

impl TurnSum {
    /// Every turn of the game has the number.
    pub fn complete(&self) -> bool {
        self.turns == self.of
    }
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
    /// The game's turns, from the first to the last the log has (turns the
    /// log skipped count): the `of` of every [`TurnSum`].
    pub turns: u32,
    /// The gold numbers: None only when no turn has the number (an older
    /// game, or no turn's gold in the log); else the sum over the turns
    /// that have it, and how many they are.
    pub gold_spent: Option<TurnSum>,
    pub actions: u32,
    // Parser revision 4 (T-210).
    pub extra_gold: Option<TurnSum>,
    pub sell_gold: Option<TurnSum>,
    pub buy_gold: Option<TurnSum>,
    pub spell_gold: Option<TurnSum>,
    /// Gold beyond the fixed income: extra gold plus sell gold, over the
    /// turns that have both.
    pub gold_beyond_income: Option<TurnSum>,
    pub free_refreshes: Option<TurnSum>,
    pub passes: Option<TurnSum>,
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
    let minutes = to_ms.saturating_sub(from_ms) as f64 / MS_PER_MINUTE;
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
    let seconds =
        (t.end_ms > t.start_ms).then(|| t.end_ms.saturating_sub(t.start_ms) as f64 / 1000.0);
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

/// The game's turns: up to the last one the log has, gaps included.
fn turn_count(record: &ShopRecord) -> u32 {
    let last = record.turns.iter().map(|t| t.turn).max().unwrap_or(0);
    let last = u32::try_from(last.clamp(0, MAX_TURN)).unwrap_or(0);
    last.max(u32::try_from(record.turns.len()).unwrap_or(u32::MAX))
}

/// A number summed over the turns that have it; None when none has it.
fn sum_known(record: &ShopRecord, f: impl Fn(&ShopTurn) -> Option<i64>) -> Option<TurnSum> {
    let mut sum = TurnSum {
        of: turn_count(record),
        ..TurnSum::default()
    };
    for value in record.turns.iter().filter_map(f) {
        sum.value = sum.value.saturating_add(value);
        sum.turns += 1;
    }
    (sum.turns > 0).then_some(sum)
}

pub fn totals(record: &ShopRecord) -> ShopTotals {
    let sum =
        |f: &dyn Fn(&ShopTurn) -> u32| record.turns.iter().map(f).fold(0, u32::saturating_add);
    let gold = |f: fn(&ShopTurn) -> Option<u32>| sum_known(record, move |t| f(t).map(i64::from));
    ShopTotals {
        rolls: sum(&|t| t.rolls),
        free_rolls: sum(&|t| t.free_rolls),
        buys: sum(&|t| t.buys.len() as u32),
        spell_buys: sum(&|t| t.spell_buys),
        sells: sum(&|t| t.sells.len() as u32),
        freezes: sum(&|t| t.freezes),
        turns: turn_count(record),
        gold_spent: sum_known(record, |t| t.gold_spent),
        actions: record.actions.len() as u32,
        extra_gold: gold(|t| t.extra_gold),
        sell_gold: gold(|t| t.sell_gold),
        buy_gold: gold(|t| t.buy_gold),
        spell_gold: gold(|t| t.spell_gold),
        gold_beyond_income: sum_known(record, |t| {
            Some(i64::from(t.extra_gold?) + i64::from(t.sell_gold?))
        }),
        free_refreshes: gold(|t| t.free_refreshes),
        passes: sum_known(record, |t| t.passes.as_ref().map(|p| p.len() as i64)),
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
        let next = turns.last().map_or(1, |v| v.turn.saturating_add(1));
        turns.extend((next..t.turn.min(MAX_TURN)).map(gap));
        let tier_up = record.tier_ups.iter().any(|u| u.turn == t.turn);
        turns.push(turn_view(t, tier_up, logged));
    }
    let has_turns = !record.turns.is_empty();
    let minutes = match (record.start_ms, record.end_ms) {
        (Some(start), Some(end)) if end > start => {
            Some(end.saturating_sub(start) as f64 / MS_PER_MINUTE)
        }
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

/// Turns behind an average: `known` of the `of` turns of the games counted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct TurnCount {
    pub known: u64,
    pub of: u64,
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
    /// Per game, over the turns whose gold the log has.
    pub gold_spent: Option<f64>,
    /// Games with at least one turn's gold in the log: the denominator of
    /// `gold_spent`; `gold_turns` says how many of their turns back it.
    pub gold_games: usize,
    pub gold_turns: TurnCount,
    pub apm: Option<f64>,
    /// Games with a time span in the log: the denominator of `apm`.
    pub apm_games: usize,
    /// Per game: gold beyond the turn's own and beyond sells, gold from
    /// sells, both together, free rolls used, cards passed (T-210), each
    /// over the turns that have it.
    pub extra_gold: Option<f64>,
    pub sell_gold: Option<f64>,
    pub gold_beyond_income: Option<f64>,
    pub free_refreshes: Option<f64>,
    pub passes: Option<f64>,
    /// Games with the gold extras in at least one turn: the denominator of
    /// the gold extras (games saved before parser revision 4 have none).
    pub extras_games: usize,
    pub extras_turns: TurnCount,
    /// Games with the passes in at least one turn: the denominator of `passes`.
    pub pass_games: usize,
    /// Games whose log gives the roll button's free rolls: the denominator
    /// of `free_refreshes`.
    pub free_refresh_games: usize,
    pub tier_turns: Vec<TierTurn>,
    pub source: Source,
    pub apm_definition: &'static str,
}

/// A per-game average over the games that have the number in some turn.
#[derive(Debug, Default)]
struct Avg {
    sum: i64,
    games: usize,
    turns: TurnCount,
}

impl Avg {
    fn add(&mut self, value: Option<TurnSum>) {
        if let Some(v) = value {
            self.sum = self.sum.saturating_add(v.value);
            self.games += 1;
            self.turns.known += u64::from(v.turns);
            self.turns.of += u64::from(v.of);
        }
    }

    fn average(&self) -> Option<f64> {
        (self.games > 0).then(|| self.sum as f64 / self.games as f64)
    }
}

/// Sums for [`ShopStats`], filled game by game.
#[derive(Debug, Default)]
pub struct ShopCounts {
    stats: ShopStats,
    rolls: u64,
    free_rolls: u64,
    buys: u64,
    sells: u64,
    apm: f64,
    gold: Avg,
    extra_gold: Avg,
    sell_gold: Avg,
    beyond: Avg,
    free_refreshes: Avg,
    passes: Avg,
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
        self.gold.add(t.gold_spent);
        if let Some(apm) = game_apm(&record) {
            self.apm += apm;
            self.stats.apm_games += 1;
        }
        self.extra_gold.add(t.extra_gold);
        self.sell_gold.add(t.sell_gold);
        self.beyond.add(t.gold_beyond_income);
        self.passes.add(t.passes);
        self.free_refreshes.add(t.free_refreshes);
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
            gold_spent: self.gold.average(),
            gold_games: self.gold.games,
            gold_turns: self.gold.turns,
            apm: avg(self.apm, self.stats.apm_games),
            extra_gold: self.extra_gold.average(),
            sell_gold: self.sell_gold.average(),
            gold_beyond_income: self.beyond.average(),
            extras_games: self.extra_gold.games,
            extras_turns: self.extra_gold.turns,
            free_refreshes: self.free_refreshes.average(),
            free_refresh_games: self.free_refreshes.games,
            passes: self.passes.average(),
            pass_games: self.passes.games,
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
            (t.rolls, t.free_rolls, t.buys, t.actions, t.turns),
            (4, 2, 2, 3, 2)
        );
        assert_eq!(
            t.gold_spent,
            Some(TurnSum {
                value: 6,
                turns: 2,
                of: 2
            })
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
            (v(t.extra_gold), v(t.sell_gold), v(t.gold_beyond_income)),
            (Some(2), Some(4), Some(6))
        );
        assert_eq!((v(t.buy_gold), v(t.spell_gold)), (Some(6), Some(0)));
        assert_eq!((v(t.free_refreshes), v(t.passes)), (Some(2), Some(1)));
        assert!(t.extra_gold.unwrap().complete());
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
    }

    fn v(sum: Option<TurnSum>) -> Option<i64> {
        sum.map(|s| s.value)
    }

    /// Turn 1 as the log gave it in the user's game of 2026-10-11: the
    /// turn is there, its gold is not.
    fn no_gold(n: i64) -> Value {
        let mut t = turn4(n, 0, 0, &[]);
        for key in [
            "gold",
            "gold_spent",
            "extra_gold",
            "sell_gold",
            "buy_gold",
            "spell_gold",
        ] {
            t[key] = Value::Null;
        }
        t
    }

    #[test]
    fn a_turn_without_its_gold_leaves_the_others_summed_and_says_how_many() {
        let r = shop_recap(&report(
            vec![no_gold(1), turn4(2, 2, 1, &[]), turn4(3, 0, 3, &[])],
            3,
            true,
        ));
        let t = r.totals.unwrap();
        let known = |value| {
            Some(TurnSum {
                value,
                turns: 2,
                of: 3,
            })
        };
        assert_eq!(t.gold_spent, known(6));
        assert_eq!((t.extra_gold, t.sell_gold), (known(2), known(4)));
        assert_eq!(t.gold_beyond_income, known(6));
        assert!(!t.gold_spent.unwrap().complete());
        // The free rolls and passes are kept apart from the gold: all 3 turns.
        assert_eq!(t.free_refreshes.map(|s| (s.turns, s.of)), Some((3, 3)));
        // The missing turn stays unknown in the turn list: never a made-up 0.
        assert_eq!((r.turns[0].gold_spent, r.turns[0].extra_gold), (None, None));
    }

    #[test]
    fn a_game_with_no_turn_gold_has_no_gold_totals() {
        let r = shop_recap(&report(vec![no_gold(1), no_gold(2)], 2, true));
        let t = r.totals.unwrap();
        assert_eq!(
            (t.gold_spent, t.extra_gold, t.gold_beyond_income),
            (None, None, None)
        );
        assert_eq!(t.rolls, 4);
        let mut counts = ShopCounts::default();
        counts.add(&report(vec![no_gold(1), no_gold(2)], 2, true));
        let s = counts.finish();
        assert_eq!((s.games, s.gold_games, s.gold_spent), (1, 0, None));
        assert_eq!((s.extras_games, s.extra_gold), (0, None));
    }

    #[test]
    fn a_turn_the_log_skipped_counts_in_the_turns_of_the_game() {
        let r = shop_recap(&report(
            vec![turn4(1, 0, 0, &[]), turn4(3, 0, 0, &[])],
            2,
            true,
        ));
        let t = r.totals.unwrap();
        assert_eq!(t.turns, 3);
        assert_eq!(t.gold_spent.map(|s| (s.turns, s.of)), Some((2, 3)));
    }

    #[test]
    fn stats_count_a_game_missing_one_turn_and_say_how_many_turns() {
        let mut counts = ShopCounts::default();
        counts.add(&report(vec![no_gold(1), turn4(2, 2, 1, &[])], 1, true));
        counts.add(&report(
            vec![turn4(1, 4, 3, &[]), turn4(2, 0, 1, &[])],
            1,
            true,
        ));
        let s = counts.finish();
        assert_eq!((s.gold_games, s.extras_games, s.pass_games), (2, 2, 2));
        assert_eq!(s.gold_turns, TurnCount { known: 3, of: 4 });
        assert_eq!(s.extras_turns, TurnCount { known: 3, of: 4 });
        assert_eq!(s.gold_spent, Some(4.5));
        assert_eq!((s.extra_gold, s.sell_gold), (Some(3.0), Some(2.5)));
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

    #[test]
    fn times_and_counts_at_the_ends_of_their_range_do_not_overflow() {
        let mut far = turn(i64::MAX, i64::MIN, i64::MAX, u32::MAX);
        far["rolls"] = json!(u32::MAX);
        let mut rep = report(vec![turn(1, 0, 60_000, 6), far.clone(), far], 6, true);
        rep["shop"]["start_ms"] = json!(i64::MIN);
        rep["shop"]["end_ms"] = json!(i64::MAX);
        let r = shop_recap(&rep);
        assert!(r.turns.len() <= 103);
        assert_eq!(r.totals.map(|t| t.rolls), Some(u32::MAX));
        let mut counts = ShopCounts::default();
        counts.add(&rep);
        counts.add(&rep);
        let _ = counts.finish();
    }
}
