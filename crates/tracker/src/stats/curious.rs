//! Curious stats (T-216): fun totals over every game in the history, each
//! with its source and the number of games that back it. The window only
//! renders them.
//!
//! Unknown is not zero: a stat no game backs has no value and says why, and
//! a game saved before a field existed is left out of that stat's count
//! ("in 14 of 29 games"), never counted as 0. A stat the saved data cannot
//! back at all is listed in `dropped` with the reason, not guessed.

use std::collections::BTreeMap;

use bg_parser::collector::is_golden;
use bg_parser::shop::ShopRecord;
use serde::Serialize;
use serde_json::Value;

use crate::choices::own_triples;
use crate::provenance::Source;
use crate::recap::{combats_in_order, own_health_points, Outcome};
use crate::shop::{record_of, totals};

/// Card names (English, from the card data) that count as golden Brann or
/// Titus when sold.
const BRANN_AND_TITUS: [&str; 2] = ["Brann Bronzebeard", "Titus Rivendare"];

const WHY_NO_SHOP: &str = "No game has a shop record. Games saved before the app kept the shop need tavern-watch --reparse while their logs are on disk.";
const WHY_NO_GOLD_DETAIL: &str = "No game has this gold record yet: it is kept since parser revision 4. Older games need tavern-watch --reparse while their logs are on disk.";
const WHY_NO_CARD_DATA: &str =
    "The card data is not on this PC yet, so the sold minions cannot be named.";
const WHY_UNNAMED: &str = "Every game with a shop record sold a golden minion the card data does not know, so it cannot be told.";
const WHY_NO_BUYS: &str = "No minion bought in the games with a shop record.";
const WHY_NO_RESULTS: &str = "No combat result is known in any game.";
const WHY_NO_HEALTH: &str = "No game has your health before and after the same combat.";

const BASIS_GOLD_SPENT: &str = "Gold you spent in the tavern, summed over every turn the log has.";
const BASIS_EXTRA_GOLD: &str = "Gold beyond each turn's own: from cards, trinkets, refunds and the like (the game's temporary gold). Gold from selling is apart.";
const BASIS_SELL_GOLD: &str = "Gold you got for selling minions.";
const BASIS_PAID_ROLLS: &str = "Rolls that spent gold.";
const BASIS_FREE_ROLLS: &str = "Rolls that spent no gold.";
const BASIS_BRANN_TITUS: &str = "Golden minions you sold whose card name is Brann Bronzebeard or Titus Rivendare. The sells are from the log; the names from the card database.";
const BASIS_MOST_BOUGHT: &str = "The minion you bought most often over all games, by card id: a golden copy bought from the tavern counts as its own card (a tie takes the first card id in alphabetical order).";
const BASIS_WIN_STREAK: &str = "Most combats won in a row inside one game. A tie, a loss, an unknown result or a round the log lacks ends the streak. Inferred when any result in it was worked out from health.";
const BASIS_DAMAGE: &str = "Your health before a combat minus your health after it (armor included), the largest over all games. Inferred: health paid or healed during the shop turn before the combat is counted in it too, and a combat with no health after it in the log (often the last one) is left out.";

const WHY_NO_TRIPLES: &str = "No finished game has the leaderboard's triple count yet: it is kept since parser revision 5. Older games need tavern-watch --reparse while their logs are on disk.";
const BASIS_TRIPLES: &str = "Triples you made, as the game's leaderboard counted them at the end of each finished game (golden minions from cards or discovers are not triples). In Duos it is your own hero's count.";

/// Stats the saved data cannot back. Triples left this list once the
/// leaderboard's count was kept (parser revision 5, D-058).
const DROPPED: [(&str, &str); 0] = [];

/// Looks a card id up in the card data: its English name, None when unknown.
pub type CardNames<'a> = &'a dyn Fn(&str) -> Option<String>;

/// One curious stat.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CuriousStat {
    /// Stable id for the window.
    pub key: &'static str,
    pub label: &'static str,
    /// None when no game backs it (`missing` says why).
    pub value: Option<i64>,
    /// The card the stat is about, by card id (the most bought minion).
    pub card: Option<String>,
    pub source: Source,
    /// Games whose record backs the value: "in `games` of `Curious::games`".
    pub games: usize,
    pub missing: Option<&'static str>,
    /// How the value is worked out.
    pub basis: &'static str,
}

/// A stat left out because the data cannot back it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Dropped {
    pub label: &'static str,
    pub why: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Curious {
    /// Every game counted (the denominator of each stat's `games`).
    pub games: usize,
    pub stats: Vec<CuriousStat>,
    pub dropped: Vec<Dropped>,
}

/// A sum over the games that have the number.
#[derive(Default)]
struct Sum {
    total: i64,
    games: usize,
}

impl Sum {
    fn add(&mut self, value: Option<i64>) {
        if let Some(v) = value {
            self.total = self.total.saturating_add(v);
            self.games += 1;
        }
    }

    fn stat(
        &self,
        key: &'static str,
        label: &'static str,
        basis: &'static str,
        why: &'static str,
    ) -> CuriousStat {
        let known = self.games > 0;
        CuriousStat {
            key,
            label,
            value: known.then_some(self.total),
            card: None,
            source: Source::log_if(known),
            games: self.games,
            missing: (!known).then_some(why),
            basis,
        }
    }
}

/// The longest run of won combats, and whether every result in it is the
/// game's own.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
struct Streak {
    len: i64,
    from_log: bool,
}

impl Streak {
    /// Longer wins; at the same length, one from the log beats an inferred one.
    fn better(self, other: Streak) -> Streak {
        if (other.len, other.from_log) > (self.len, self.from_log) {
            other
        } else {
            self
        }
    }
}

/// The longest win streak of one game; None when it has no known result.
fn game_streak(report: &Value) -> Option<Streak> {
    let combats = combats_in_order(report);
    if combats.iter().all(|c| c.outcome == Outcome::Unknown) {
        return None;
    }
    let mut best = Streak::default();
    let mut run = Streak::default();
    let mut last_round = None;
    for c in &combats {
        let follows = last_round.is_some_and(|r: i64| c.round == r + 1);
        last_round = Some(c.round);
        if c.outcome != Outcome::Won {
            run = Streak::default();
            continue;
        }
        run = if follows && run.len > 0 {
            Streak {
                len: run.len + 1,
                from_log: run.from_log && c.source == Source::Log,
            }
        } else {
            Streak {
                len: 1,
                from_log: c.source == Source::Log,
            }
        };
        best = best.better(run);
    }
    Some(best)
}

/// The most health lost in one combat of a game; None when no combat has
/// the health before and after it.
fn game_damage(report: &Value) -> Option<i64> {
    own_health_points(report)
        .windows(2)
        .filter(|pair| pair[1].round == pair[0].round + 1)
        .filter_map(|pair| Some((pair[0].health? - pair[1].health?).max(0)))
        .max()
}

/// Golden Brann and Titus sold in one game; None when a golden minion sold
/// there is not in the card data (that game cannot be told).
fn brann_titus_sold(record: &ShopRecord, names: CardNames<'_>) -> Option<i64> {
    let mut sold = 0;
    for id in record.turns.iter().flat_map(|t| &t.sells).flatten() {
        if !is_golden(Some(id)) {
            continue;
        }
        let name = names(id)?;
        if BRANN_AND_TITUS.contains(&name.as_str()) {
            sold += 1;
        }
    }
    Some(sold)
}

#[derive(Default)]
struct Counts {
    games: usize,
    gold_spent: Sum,
    extra_gold: Sum,
    sell_gold: Sum,
    paid_rolls: Sum,
    free_rolls: Sum,
    brann_titus: Sum,
    triples: Sum,
    shop_games: usize,
    bought: BTreeMap<String, i64>,
    streak: Option<Streak>,
    streak_games: usize,
    damage: Option<i64>,
    damage_games: usize,
}

impl Counts {
    fn add(&mut self, report: &Value, names: Option<CardNames<'_>>) {
        self.games += 1;
        if let Some(record) = record_of(report).ok().filter(|r| !r.turns.is_empty()) {
            let t = totals(&record);
            self.shop_games += 1;
            self.gold_spent.add(t.gold_spent);
            self.extra_gold.add(t.extra_gold);
            self.sell_gold.add(t.sell_gold);
            self.paid_rolls
                .add(Some(i64::from(t.rolls.saturating_sub(t.free_rolls))));
            self.free_rolls.add(Some(i64::from(t.free_rolls)));
            if let Some(names) = names {
                self.brann_titus.add(brann_titus_sold(&record, names));
            }
            for id in record.turns.iter().flat_map(|t| &t.buys).flatten() {
                *self.bought.entry(id.clone()).or_default() += 1;
            }
        }
        self.triples.add(own_triples(report));
        if let Some(streak) = game_streak(report) {
            self.streak_games += 1;
            self.streak = Some(self.streak.unwrap_or_default().better(streak));
        }
        if let Some(damage) = game_damage(report) {
            self.damage_games += 1;
            self.damage = Some(self.damage.map_or(damage, |d| d.max(damage)));
        }
    }

    fn most_bought(&self) -> CuriousStat {
        // Most buys; ties take the first id (the map is in id order).
        let top = self
            .bought
            .iter()
            .fold(None::<(&String, i64)>, |best, (id, &n)| match best {
                Some((_, m)) if m >= n => best,
                _ => Some((id, n)),
            });
        let why = if self.shop_games == 0 {
            WHY_NO_SHOP
        } else {
            WHY_NO_BUYS
        };
        CuriousStat {
            key: "most_bought",
            label: "Most bought minion",
            value: top.map(|(_, n)| n),
            card: top.map(|(id, _)| id.clone()),
            source: Source::log_if(top.is_some()),
            games: self.shop_games,
            missing: top.is_none().then_some(why),
            basis: BASIS_MOST_BOUGHT,
        }
    }

    fn brann_titus(&self, names_known: bool) -> CuriousStat {
        let mut stat = self.brann_titus.stat(
            "golden_brann_titus_sold",
            "Golden Brann or Titus sold",
            BASIS_BRANN_TITUS,
            match (names_known, self.shop_games) {
                (false, _) => WHY_NO_CARD_DATA,
                (true, 0) => WHY_NO_SHOP,
                (true, _) => WHY_UNNAMED,
            },
        );
        if stat.value.is_some() {
            stat.source = Source::CardData;
        }
        stat
    }

    fn finish(self, names_known: bool) -> Curious {
        let streak = self.streak.map(|s| CuriousStat {
            key: "longest_win_streak",
            label: "Longest win streak (combats)",
            value: Some(s.len),
            card: None,
            source: if s.from_log || s.len == 0 {
                Source::Log
            } else {
                Source::Inferred
            },
            games: self.streak_games,
            missing: None,
            basis: BASIS_WIN_STREAK,
        });
        let damage = CuriousStat {
            key: "most_damage_one_combat",
            label: "Most damage taken in one combat",
            value: self.damage,
            card: None,
            source: if self.damage.is_some() {
                Source::Inferred
            } else {
                Source::Unknown
            },
            games: self.damage_games,
            missing: self.damage.is_none().then_some(WHY_NO_HEALTH),
            basis: BASIS_DAMAGE,
        };
        let shop_why = WHY_NO_SHOP;
        Curious {
            games: self.games,
            stats: vec![
                self.gold_spent
                    .stat("gold_spent", "Gold spent", BASIS_GOLD_SPENT, shop_why),
                self.extra_gold.stat(
                    "extra_gold",
                    "Gold beyond the fixed income",
                    BASIS_EXTRA_GOLD,
                    WHY_NO_GOLD_DETAIL,
                ),
                self.sell_gold.stat(
                    "sell_gold",
                    "Gold from selling",
                    BASIS_SELL_GOLD,
                    WHY_NO_GOLD_DETAIL,
                ),
                self.brann_titus(names_known),
                self.paid_rolls.stat(
                    "paid_rolls",
                    "Rolls paid with gold",
                    BASIS_PAID_ROLLS,
                    shop_why,
                ),
                self.free_rolls
                    .stat("free_rolls", "Free rolls", BASIS_FREE_ROLLS, shop_why),
                self.most_bought(),
                self.triples
                    .stat("triples", "Triples", BASIS_TRIPLES, WHY_NO_TRIPLES),
                streak.unwrap_or(CuriousStat {
                    key: "longest_win_streak",
                    label: "Longest win streak (combats)",
                    value: None,
                    card: None,
                    source: Source::Unknown,
                    games: 0,
                    missing: Some(WHY_NO_RESULTS),
                    basis: BASIS_WIN_STREAK,
                }),
                damage,
            ],
            dropped: DROPPED
                .iter()
                .map(|&(label, why)| Dropped { label, why })
                .collect(),
        }
    }
}

/// Curious stats over these reports (the latest of every counted game).
/// `names` looks a card id up in the card data; None when there is none.
pub fn compute<'a>(
    reports: impl IntoIterator<Item = &'a Value>,
    names: Option<CardNames<'_>>,
) -> Curious {
    let mut counts = Counts::default();
    for report in reports {
        counts.add(report, names);
    }
    counts.finish(names.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// A shop turn as the parser of revision 4 writes it.
    fn turn(n: i64, spent: i64, rolls: u32, free: u32, buys: &[&str], sells: &[&str]) -> Value {
        json!({
            "turn": n, "start_ms": n * 1000, "end_ms": n * 1000 + 900, "tier": 1, "gold": 3 + n,
            "gold_spent": spent, "rolls": rolls, "free_rolls": free, "buys": buys,
            "spell_buys": 0, "sells": sells, "freezes": 0, "offers": [], "actions": 0,
            "extra_gold": 2, "sell_gold": sells.len(), "buy_gold": 0, "spell_gold": 0,
            "free_refreshes": free, "passes": null,
        })
    }

    fn game(turns: Vec<Value>) -> Value {
        json!({
            "status": "ok", "game_type": "GT_BATTLEGROUNDS", "hero": "H", "final_place": 1,
            "shop": {"turns": turns, "tier_ups": [], "actions": [], "start_ms": 0,
                     "end_ms": 99_000, "ended": true},
        })
    }

    /// A revision 3 turn: no gold detail.
    fn old_turn(n: i64) -> Value {
        let mut t = turn(n, 3, 1, 0, &["BG_A"], &[]);
        for key in [
            "extra_gold",
            "sell_gold",
            "buy_gold",
            "spell_gold",
            "free_refreshes",
            "passes",
        ] {
            t.as_object_mut().unwrap().remove(key);
        }
        t
    }

    /// A Solo game with combats: each is (result as the game recorded it,
    /// own health after). We are player 1, always against player 2.
    fn fights(rounds: &[(Option<&str>, i64)]) -> Value {
        let rounds: Vec<Value> = rounds
            .iter()
            .enumerate()
            .map(|(i, (result, own))| {
                json!({
                    "number": i + 1, "own_health_after": own, "result": result,
                    "health_after": {"1": own, "2": 30},
                    "entries": [{"side": "opponent", "player_id": 2}],
                })
            })
            .collect();
        json!({
            "status": "ok", "game_type": "GT_BATTLEGROUNDS", "local_player_id": 1,
            "hero": "H", "final_place": 2,
            "lobby": [{"player_id": 1, "hero": "H"}, {"player_id": 2, "hero": "O"}],
            "start_health": {"1": 30, "2": 30}, "rounds": rounds,
        })
    }

    fn stat<'a>(c: &'a Curious, key: &str) -> &'a CuriousStat {
        c.stats.iter().find(|s| s.key == key).unwrap()
    }

    fn no_names() -> Option<CardNames<'static>> {
        None
    }

    #[test]
    fn shop_totals_add_up_over_games_with_their_count() {
        let games = [
            game(vec![
                turn(1, 3, 2, 1, &["BG_A"], &[]),
                turn(2, 4, 1, 0, &["BG_B", "BG_A"], &["BG_A"]),
            ]),
            game(vec![turn(1, 3, 0, 0, &["BG_B"], &[])]),
            json!({"status": "ok", "game_type": "GT_BATTLEGROUNDS"}), // no shop: not recorded
        ];
        let c = compute(&games, no_names());
        assert_eq!(c.games, 3);
        let gold = stat(&c, "gold_spent");
        assert_eq!(
            (gold.value, gold.games, gold.source),
            (Some(10), 2, Source::Log)
        );
        assert_eq!(
            stat(&c, "extra_gold").value,
            Some(6),
            "2 per turn, three turns"
        );
        assert_eq!(stat(&c, "sell_gold").value, Some(1));
        assert_eq!(stat(&c, "paid_rolls").value, Some(2));
        assert_eq!(stat(&c, "free_rolls").value, Some(1));
        let most = stat(&c, "most_bought");
        assert_eq!(
            (most.card.as_deref(), most.value),
            (Some("BG_A"), Some(2)),
            "tie: first id"
        );
    }

    #[test]
    fn older_records_are_left_out_of_the_count_not_added_as_zero() {
        let games = [
            game(vec![old_turn(1)]),
            game(vec![turn(1, 3, 0, 0, &[], &[])]),
        ];
        let c = compute(&games, no_names());
        let extra = stat(&c, "extra_gold");
        assert_eq!((extra.value, extra.games), (Some(2), 1), "in 1 of 2 games");
        assert_eq!(stat(&c, "gold_spent").games, 2);
    }

    #[test]
    fn a_stat_no_game_backs_says_why_instead_of_zero() {
        let c = compute(&[json!({"status": "ok"})], no_names());
        for s in &c.stats {
            assert_eq!(s.value, None, "{}", s.key);
            assert_eq!(s.source, Source::Unknown, "{}", s.key);
            assert_eq!(s.games, 0);
            assert!(s.missing.is_some(), "{} says why", s.key);
        }
        let old = compute(&[game(vec![old_turn(1)])], no_names());
        assert_eq!(stat(&old, "extra_gold").missing, Some(WHY_NO_GOLD_DETAIL));
        assert_eq!(stat(&old, "gold_spent").value, Some(3));
    }

    #[test]
    fn empty_history_has_every_stat_unknown_and_lists_the_dropped_ones() {
        let c = compute(&[], no_names());
        assert_eq!(c.games, 0);
        assert!(c.stats.iter().all(|s| s.value.is_none()));
        assert!(c.dropped.is_empty(), "triples are backed since revision 5");
        assert_eq!(stat(&c, "triples").missing, Some(WHY_NO_TRIPLES));
    }

    #[test]
    fn triples_sum_the_leaderboard_count_of_finished_games_only() {
        let with = |triples: Value, status: &str| {
            json!({
                "status": status, "game_type": "GT_BATTLEGROUNDS", "local_player_id": 1,
                "lobby": [{"player_id": 1, "hero": "H", "triples": triples, "trinkets": []}],
            })
        };
        let games = [
            with(
                json!([{"turn": 4, "count": 1}, {"turn": 8, "count": 3}]),
                "ok",
            ),
            with(json!([]), "ok"),
            with(json!([{"turn": 4, "count": 1}]), "incomplete"),
            json!({"status": "ok", "game_type": "GT_BATTLEGROUNDS", "local_player_id": 1,
                   "lobby": [{"player_id": 1, "hero": "H"}]}),
        ];
        let c = compute(&games, no_names());
        let t = stat(&c, "triples");
        assert_eq!((t.value, t.games, t.source), (Some(3), 2, Source::Log));
    }

    #[test]
    fn golden_brann_and_titus_are_named_by_the_card_data() {
        let games = [
            game(vec![turn(
                1,
                3,
                0,
                0,
                &[],
                &["BRANN_G", "TB_BaconUps_TITUS", "BRANN", "OTHER_G"],
            )]),
            game(vec![turn(1, 3, 0, 0, &[], &["UNKNOWN_G"])]), // the card data does not know it
        ];
        let names = |id: &str| match id {
            "BRANN_G" | "BRANN" => Some("Brann Bronzebeard".to_string()),
            "TB_BaconUps_TITUS" => Some("Titus Rivendare".to_string()),
            "OTHER_G" => Some("Other".to_string()),
            _ => None,
        };
        let c = compute(&games, Some(&names as CardNames<'_>));
        let s = stat(&c, "golden_brann_titus_sold");
        assert_eq!((s.value, s.games, s.source), (Some(2), 1, Source::CardData));
        let blind = compute(&games, no_names());
        let s = stat(&blind, "golden_brann_titus_sold");
        assert_eq!((s.value, s.missing), (None, Some(WHY_NO_CARD_DATA)));
    }

    #[test]
    fn the_win_streak_counts_rounds_in_a_row_and_keeps_its_source() {
        // Won, won, lost, won, won, won: 3 in a row, all from the log.
        let a = fights(&[
            (Some("won"), 30),
            (Some("won"), 30),
            (Some("lost"), 25),
            (Some("won"), 25),
            (Some("won"), 25),
            (Some("won"), 25),
        ]);
        let c = compute(&[a], no_names());
        let s = stat(&c, "longest_win_streak");
        assert_eq!((s.value, s.source, s.games), (Some(3), Source::Log, 1));
        // An older record: results from health only, so inferred.
        let mut b = fights(&[(None, 30), (None, 30)]);
        b["rounds"][0]["health_after"]["2"] = json!(20);
        b["rounds"][1]["health_after"]["2"] = json!(10);
        let c = compute(&[b], no_names());
        let s = stat(&c, "longest_win_streak");
        assert_eq!((s.value, s.source), (Some(2), Source::Inferred));
    }

    #[test]
    fn an_unknown_result_or_a_missing_round_ends_a_streak() {
        let mut g = fights(&[
            (Some("won"), 30),
            (None, 25),
            (Some("won"), 25),
            (Some("won"), 25),
        ]);
        // Round 2 both sides lost health: unknown.
        g["rounds"][1]["health_after"]["2"] = json!(20);
        let c = compute(&[g.clone()], no_names());
        assert_eq!(stat(&c, "longest_win_streak").value, Some(2));
        // Round 3 is not in the log: rounds 2 and 4 do not follow each other.
        let mut gap = fights(&[(Some("won"), 30), (Some("won"), 30), (Some("won"), 30)]);
        gap["rounds"][2]["number"] = json!(4);
        let c = compute(&[gap], no_names());
        assert_eq!(stat(&c, "longest_win_streak").value, Some(2));
    }

    #[test]
    fn games_with_no_known_result_do_not_back_the_streak() {
        let c = compute(&[fights(&[])], no_names());
        let s = stat(&c, "longest_win_streak");
        assert_eq!((s.value, s.games), (None, 0));
        let lost = compute(&[fights(&[(Some("lost"), 20)])], no_names());
        assert_eq!(stat(&lost, "longest_win_streak").value, Some(0), "a real 0");
    }

    #[test]
    fn most_damage_is_the_largest_health_drop_and_inferred() {
        let a = fights(&[(Some("lost"), 22), (Some("won"), 22), (Some("lost"), 9)]);
        let mut b = fights(&[(Some("lost"), 18)]);
        b["start_health"]["1"] = json!(40);
        let c = compute(&[a, b], no_names());
        let s = stat(&c, "most_damage_one_combat");
        assert_eq!(
            (s.value, s.games, s.source),
            (Some(22), 2, Source::Inferred)
        );
    }

    #[test]
    fn hand_edited_records_do_not_break_the_curious_stats() {
        let games = [
            json!("x"),
            json!({"shop": 3, "rounds": "no"}),
            json!({"shop": null}),
        ];
        let c = compute(&games, no_names());
        assert_eq!(c.games, 3);
        assert!(c.stats.iter().all(|s| s.value.is_none()));
    }
}
