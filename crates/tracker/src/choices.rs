//! The season's choices and the leaderboard's extras (T-207, T-211, T-212,
//! D-058), for the recap and the stats; the window only renders them.
//!
//! - Every choice the player was offered (discovers, trinket offers, Dark
//!   Gifts, quests, triple rewards) with what they took, sorted by what
//!   offered it (its source card).
//! - The game's Old God, by database id.
//! - Each lobby hero's triples and trinkets as the leaderboard showed them,
//!   by seat and hero, never by name.
//! - Stats per trinket and per Dark Gift: offered, taken, pick rate and the
//!   average place when taken.
//!
//! Only the player's own choices: the log has no one else's (D-004). Unknown
//! is not zero: a game saved before parser revision 5 says "not recorded",
//! an unreadable game "no data", and neither is counted.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use serde_json::Value;

use crate::provenance::Source;
use crate::recap::text;

/// Bounds of the parser (discover.rs, leaderboard.rs): more is not a record
/// it wrote.
const MAX_DISCOVERS: usize = 200;
const MAX_OPTIONS: usize = 10;
const MAX_CHANGES: usize = 64;
const MAX_DBF_ID: i64 = 10_000_000;

const TRINKET_SOURCES: [&str; 2] = ["BG30_Trinket_1st", "BG30_Trinket_2nd"];
const DARK_GIFT_SOURCE: &str = "BG36_MidGameEffect_010";
const TRIPLE_REWARD: &str = "TB_BaconShop_Triples_01";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChoiceState {
    Recorded,
    /// Saved by a parser older than revision 5: re-read with `--reparse`.
    NotRecorded,
    /// The game could not be read.
    NoData,
}

/// What offered a choice, from its source card.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChoiceKind {
    /// The trinket offers (`BG30_Trinket_1st`, `_2nd`).
    Trinket,
    /// Dark Discovery: minions, each with a Dark Gift.
    DarkGift,
    /// A quest and its reward.
    Quest,
    /// A triple reward's discover.
    TripleReward,
    /// Any other discover (hero powers, cards, spells).
    Discover,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ChoiceOption {
    pub card_id: Option<String>,
    /// The Dark Gift a minion came with.
    pub gift: Option<String>,
}

/// One choice the player was offered.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Choice {
    pub turn: i64,
    pub kind: ChoiceKind,
    pub source_card: Option<String>,
    pub offered: Vec<ChoiceOption>,
    /// Index in `offered` of what was taken; None when the log does not say.
    pub picked: Option<usize>,
}

/// A trinket the leaderboard showed for a hero, from a turn on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TrinketShown {
    pub turn: i64,
    /// `first`, `second` or `hero_power`.
    pub slot: String,
    /// Database id (the window turns it into a card with the card data).
    pub dbf_id: i64,
}

/// One lobby hero as the leaderboard showed it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SeatExtras {
    pub seat: i64,
    pub hero: Option<String>,
    pub own: bool,
    /// Triples at the end of the game (0 when the leaderboard never showed one).
    pub triples: i64,
    /// (turn, count) each time the count changed.
    pub triples_by_turn: Vec<(i64, i64)>,
    pub trinkets: Vec<TrinketShown>,
}

/// The choices part of a game's recap.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GameChoices {
    pub state: ChoiceState,
    pub source: Source,
    pub choices: Vec<Choice>,
    pub old_god_dbf: Option<i64>,
    pub old_god_source: Source,
    /// Every lobby hero's triples and trinkets; empty when not recorded.
    pub leaderboard: Vec<SeatExtras>,
    pub leaderboard_state: ChoiceState,
}

fn kind_of(source: Option<&str>, offered: &[ChoiceOption]) -> ChoiceKind {
    match source {
        Some(s) if TRINKET_SOURCES.contains(&s) => ChoiceKind::Trinket,
        Some(DARK_GIFT_SOURCE) => ChoiceKind::DarkGift,
        Some(TRIPLE_REWARD) => ChoiceKind::TripleReward,
        _ if offered
            .iter()
            .any(|o| o.card_id.as_deref().is_some_and(|c| c.contains("_Quest_"))) =>
        {
            ChoiceKind::Quest
        }
        _ => ChoiceKind::Discover,
    }
}

fn readable(report: &Value) -> bool {
    matches!(
        report.get("status").and_then(Value::as_str),
        Some("ok" | "incomplete")
    )
}

fn choice(v: &Value) -> Option<Choice> {
    let turn = v.get("turn")?.as_i64()?;
    let offered: Vec<ChoiceOption> = v
        .get("offered")?
        .as_array()?
        .iter()
        .take(MAX_OPTIONS)
        .map(|o| ChoiceOption {
            card_id: text(o, "card_id"),
            gift: text(o, "gift"),
        })
        .collect();
    if offered.is_empty() {
        return None;
    }
    let picked = v
        .get("picked")?
        .as_array()?
        .first()
        .and_then(Value::as_u64)
        .and_then(|i| usize::try_from(i).ok())
        .filter(|&i| i < offered.len());
    let source_card = text(v, "source");
    Some(Choice {
        turn,
        kind: kind_of(source_card.as_deref(), &offered),
        source_card,
        offered,
        picked,
    })
}

/// The player's choices of a report; Err when there are none to read.
fn read(report: &Value) -> Result<Vec<Choice>, ChoiceState> {
    let Some(list) = report.get("discovers") else {
        return Err(ChoiceState::NotRecorded);
    };
    if !readable(report) {
        return Err(ChoiceState::NoData);
    }
    let list = list.as_array().ok_or(ChoiceState::NoData)?;
    Ok(list.iter().take(MAX_DISCOVERS).filter_map(choice).collect())
}

fn seat_extras(p: &Value, own_pid: Option<i64>) -> Option<SeatExtras> {
    let seat = p.get("player_id")?.as_i64()?;
    let triples_by_turn: Vec<(i64, i64)> = p
        .get("triples")?
        .as_array()?
        .iter()
        .take(MAX_CHANGES)
        .filter_map(|t| Some((t.get("turn")?.as_i64()?, t.get("count")?.as_i64()?)))
        .collect();
    let trinkets = p
        .get("trinkets")?
        .as_array()?
        .iter()
        .take(MAX_CHANGES)
        .filter_map(|t| {
            Some(TrinketShown {
                turn: t.get("turn")?.as_i64()?,
                slot: text(t, "slot")?,
                dbf_id: t
                    .get("dbf_id")?
                    .as_i64()
                    .filter(|d| (1..=MAX_DBF_ID).contains(d))?,
            })
        })
        .collect();
    Some(SeatExtras {
        seat,
        hero: text(p, "hero"),
        own: own_pid == Some(seat),
        triples: triples_by_turn.last().map(|t| t.1).unwrap_or(0),
        triples_by_turn,
        trinkets,
    })
}

/// Every lobby hero's triples and trinkets; Err when the record has none.
fn leaderboard(report: &Value) -> Result<Vec<SeatExtras>, ChoiceState> {
    let lobby = report.get("lobby").and_then(Value::as_array);
    let Some(lobby) = lobby.filter(|l| !l.is_empty()) else {
        return Err(if readable(report) {
            ChoiceState::NoData
        } else {
            ChoiceState::NotRecorded
        });
    };
    // A record from before revision 5 has seats without the new keys.
    if lobby.iter().any(|p| p.get("triples").is_none()) {
        return Err(ChoiceState::NotRecorded);
    }
    let own = report.get("local_player_id").and_then(Value::as_i64);
    let seats: Vec<SeatExtras> = lobby.iter().filter_map(|p| seat_extras(p, own)).collect();
    if seats.is_empty() {
        return Err(ChoiceState::NoData);
    }
    Ok(seats)
}

/// The choices part of a game's recap.
pub fn game_choices(report: &Value) -> GameChoices {
    let (state, source, choices) = match read(report) {
        Ok(choices) => (ChoiceState::Recorded, Source::Log, choices),
        Err(state) => (state, Source::Unknown, Vec::new()),
    };
    let old_god_dbf = (state == ChoiceState::Recorded)
        .then(|| report.get("old_god").and_then(Value::as_i64))
        .flatten()
        .filter(|d| (1..=MAX_DBF_ID).contains(d));
    let (leaderboard_state, leaderboard) = match leaderboard(report) {
        Ok(seats) => (ChoiceState::Recorded, seats),
        Err(state) => (state, Vec::new()),
    };
    GameChoices {
        state,
        source,
        choices,
        old_god_source: Source::log_if(old_god_dbf.is_some()),
        old_god_dbf,
        leaderboard,
        leaderboard_state,
    }
}

/// One card over the choices of a kind (trinkets, Dark Gifts) in the games
/// whose choices the log has.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CardPickStats {
    pub card_id: String,
    /// Choices that offered it (a card offered twice in one choice counts once).
    pub offered: usize,
    pub picked: usize,
    pub pick_rate: Option<f64>,
    pub placed_when_picked: usize,
    pub average_place_when_picked: Option<f64>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct ChoiceStats {
    /// Games counted: their choices are in the log.
    pub games: usize,
    pub not_recorded: usize,
    pub no_data: usize,
    /// Choices of each kind over the counted games.
    pub choices_by_kind: BTreeMap<String, usize>,
    /// Offered most first.
    pub trinkets: Vec<CardPickStats>,
    /// Dark Gifts (the gift's card id), offered most first.
    pub dark_gifts: Vec<CardPickStats>,
    pub source: Source,
}

#[derive(Default)]
struct CardCounts {
    offered: usize,
    picked: usize,
    places: Vec<i64>,
}

/// Sums for [`ChoiceStats`], filled game by game.
#[derive(Default)]
pub struct ChoiceCounts {
    stats: ChoiceStats,
    trinkets: BTreeMap<String, CardCounts>,
    gifts: BTreeMap<String, CardCounts>,
}

fn count(
    cards: &mut BTreeMap<String, CardCounts>,
    offered: &[String],
    picked: Option<&String>,
    place: Option<i64>,
) {
    let unique: BTreeSet<&String> = offered.iter().collect();
    for card in unique {
        cards.entry(card.clone()).or_default().offered += 1;
    }
    if let Some(card) = picked {
        let c = cards.entry(card.clone()).or_default();
        c.picked += 1;
        c.places.extend(place);
    }
}

impl ChoiceCounts {
    /// `place` is the game's place when it counts.
    pub fn add(&mut self, report: &Value, place: Option<i64>) {
        let choices = match read(report) {
            Ok(choices) => choices,
            Err(ChoiceState::NotRecorded) => {
                self.stats.not_recorded += 1;
                return;
            }
            Err(_) => {
                self.stats.no_data += 1;
                return;
            }
        };
        self.stats.games += 1;
        for c in &choices {
            let kind = serde_json::to_value(c.kind)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default();
            *self.stats.choices_by_kind.entry(kind).or_default() += 1;
            // A choice the log does not say was answered would make every
            // offer look passed over: left out of the pick counts.
            let Some(i) = c.picked else { continue };
            match c.kind {
                ChoiceKind::Trinket => {
                    let cards: Vec<String> =
                        c.offered.iter().filter_map(|o| o.card_id.clone()).collect();
                    let picked = c.offered[i].card_id.as_ref();
                    count(&mut self.trinkets, &cards, picked, place);
                }
                ChoiceKind::DarkGift => {
                    let gifts: Vec<String> =
                        c.offered.iter().filter_map(|o| o.gift.clone()).collect();
                    let picked = c.offered[i].gift.as_ref();
                    count(&mut self.gifts, &gifts, picked, place);
                }
                _ => {}
            }
        }
    }

    pub fn finish(self) -> ChoiceStats {
        ChoiceStats {
            source: Source::log_if(self.stats.games > 0),
            trinkets: rows(self.trinkets),
            dark_gifts: rows(self.gifts),
            ..self.stats
        }
    }
}

fn rows(cards: BTreeMap<String, CardCounts>) -> Vec<CardPickStats> {
    let mut rows: Vec<CardPickStats> = cards
        .into_iter()
        .map(|(card_id, c)| {
            let placed = c.places.len();
            CardPickStats {
                card_id,
                pick_rate: (c.offered > 0).then(|| c.picked as f64 / c.offered as f64),
                average_place_when_picked: (placed > 0)
                    .then(|| c.places.iter().sum::<i64>() as f64 / placed as f64),
                placed_when_picked: placed,
                offered: c.offered,
                picked: c.picked,
            }
        })
        .collect();
    rows.sort_by(|a, b| {
        b.offered
            .cmp(&a.offered)
            .then(b.picked.cmp(&a.picked))
            .then_with(|| a.card_id.cmp(&b.card_id))
    });
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn game(discovers: Value, place: i64) -> Value {
        json!({
            "status": "ok", "game_type": "GT_BATTLEGROUNDS", "local_player_id": 2,
            "final_place": place, "old_god": 130610,
            "lobby": [
                {"player_id": 2, "hero": "H_A",
                 "triples": [{"turn": 5, "count": 1}, {"turn": 7, "count": 2}],
                 "trinkets": [{"turn": 6, "slot": "first", "dbf_id": 111}]},
                {"player_id": 3, "hero": "H_B", "triples": [], "trinkets": []},
            ],
            "discovers": discovers, "rounds": [],
        })
    }

    fn trinket(offered: &[&str], picked: usize) -> Value {
        json!({"turn": 6, "source": "BG30_Trinket_1st",
               "offered": offered.iter().map(|c| json!({"card_id": c, "gift": null})).collect::<Vec<_>>(),
               "picked": [picked]})
    }

    fn gift(gifts: &[&str], picked: usize) -> Value {
        json!({"turn": 5, "source": DARK_GIFT_SOURCE,
               "offered": gifts.iter().map(|g| json!({"card_id": "M_X", "gift": g})).collect::<Vec<_>>(),
               "picked": [picked]})
    }

    #[test]
    fn the_recap_sorts_choices_by_what_offered_them() {
        let quest = json!({"turn": 1, "source": "BG24_QuestsPlayerEnch_t",
            "offered": [{"card_id": "BG24_Quest_124", "gift": null}], "picked": [0]});
        let open = json!({"turn": 9, "source": TRIPLE_REWARD,
            "offered": [{"card_id": "M_A", "gift": null}], "picked": []});
        let other = json!({"turn": 3, "source": "BG21_HERO_000p",
            "offered": [{"card_id": "M_B", "gift": null}], "picked": [0]});
        let r = game_choices(&game(
            json!([
                quest,
                trinket(&["T_A", "T_B"], 1),
                gift(&["G_A", "G_B"], 0),
                open,
                other
            ]),
            3,
        ));
        assert_eq!((r.state, r.source), (ChoiceState::Recorded, Source::Log));
        let kinds: Vec<_> = r.choices.iter().map(|c| c.kind).collect();
        use ChoiceKind::*;
        assert_eq!(kinds, [Quest, Trinket, DarkGift, TripleReward, Discover]);
        assert_eq!(r.choices[1].picked, Some(1));
        assert_eq!(
            r.choices[3].picked, None,
            "unanswered: no pick, never a guess"
        );
        assert_eq!(
            (r.old_god_dbf, r.old_god_source),
            (Some(130610), Source::Log)
        );
        let me = &r.leaderboard[0];
        assert!(me.own);
        assert_eq!((me.triples, me.triples_by_turn.len()), (2, 2));
        assert_eq!(me.trinkets[0].dbf_id, 111);
        assert_eq!(r.leaderboard[1].triples, 0);
    }

    #[test]
    fn older_records_say_not_recorded_and_unreadable_games_no_data() {
        let mut old = game(json!([]), 1);
        let o = old.as_object_mut().unwrap();
        o.remove("discovers");
        o.remove("old_god");
        for p in old["lobby"].as_array_mut().unwrap() {
            p.as_object_mut().unwrap().remove("triples");
            p.as_object_mut().unwrap().remove("trinkets");
        }
        let r = game_choices(&old);
        assert_eq!(
            (r.state, r.source),
            (ChoiceState::NotRecorded, Source::Unknown)
        );
        assert_eq!(r.leaderboard_state, ChoiceState::NotRecorded);
        assert_eq!((r.old_god_dbf, r.old_god_source), (None, Source::Unknown));
        let mut broken = game(json!([]), 1);
        broken["status"] = json!("unsupported");
        broken["lobby"] = json!([]);
        let r = game_choices(&broken);
        assert_eq!(
            (r.state, r.leaderboard_state),
            (ChoiceState::NoData, ChoiceState::NotRecorded)
        );
    }

    #[test]
    fn stats_count_trinkets_and_gifts_offered_and_taken_with_the_place() {
        let mut counts = ChoiceCounts::default();
        let g1 = game(
            json!([trinket(&["T_A", "T_B", "T_C"], 0), gift(&["G_A", "G_B"], 1)]),
            2,
        );
        let g2 = game(json!([trinket(&["T_A", "T_D"], 1)]), 6);
        let mut old = game(json!([]), 1);
        old.as_object_mut().unwrap().remove("discovers");
        for (g, place) in [(&g1, Some(2)), (&g2, Some(6)), (&old, Some(1))] {
            counts.add(g, place);
        }
        let s = counts.finish();
        assert_eq!((s.games, s.not_recorded, s.no_data), (2, 1, 0));
        assert_eq!(s.choices_by_kind.get("trinket"), Some(&2));
        let a = &s.trinkets[0];
        assert_eq!((a.card_id.as_str(), a.offered, a.picked), ("T_A", 2, 1));
        assert_eq!(
            (a.pick_rate, a.average_place_when_picked),
            (Some(0.5), Some(2.0))
        );
        let d = s.trinkets.iter().find(|t| t.card_id == "T_D").unwrap();
        assert_eq!(d.average_place_when_picked, Some(6.0));
        let b = s.dark_gifts.iter().find(|t| t.card_id == "G_B").unwrap();
        assert_eq!((b.offered, b.picked), (1, 1));
        assert_eq!(s.source, Source::Log);
    }

    #[test]
    fn unanswered_choices_and_malformed_values_are_left_out() {
        let open = json!({"turn": 6, "source": "BG30_Trinket_2nd",
            "offered": [{"card_id": "T_A", "gift": null}], "picked": []});
        let odd = json!({"turn": 6, "source": "BG30_Trinket_2nd",
            "offered": [{"card_id": "T_A", "gift": null}], "picked": [7]});
        let empty = json!({"turn": 6, "offered": [], "picked": []});
        let mut counts = ChoiceCounts::default();
        counts.add(&game(json!([open, odd, empty]), 4), Some(4));
        let s = counts.finish();
        assert_eq!(s.games, 1);
        assert!(s.trinkets.is_empty());
        assert_eq!(s.choices_by_kind.get("trinket"), Some(&2));
    }
}
