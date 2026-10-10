//! What the game's leaderboard shows for each opponent now, for the overlay
//! (T-312, T-313): their triple count and trinkets as the leaderboard tooltip
//! shows them (parser revision 5, D-058) and the opponent the game marks as
//! next. Built from the report of the game so far, like the rest of `live`.
//!
//! Red line (D-004): only what the leaderboard shows the player at that
//! moment. The values come from the leaderboard's own hero entities, never
//! from a combat copy, and the next opponent is the game's own mark.

use serde::Serialize;
use serde_json::Value;

use crate::recap::{list, text};
use bg_parser::collector::MAX_DBF_ID;
use bg_parser::leaderboard::MAX_CHANGES;

/// Trinket slots in the order the leaderboard tooltip lists them.
const SLOTS: [&str; 3] = ["first", "second", "hero_power"];

/// A trinket the leaderboard shows now, in one slot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LiveTrinket {
    /// `first`, `second` or `hero_power` (a trinket that became a hero power).
    pub slot: String,
    /// The log's database id.
    pub dbf_id: i64,
    /// The card it is, from the card data (added by the app); None when the
    /// card data is not on this PC or does not know the id.
    pub card_id: Option<String>,
}

/// One lobby hero as the leaderboard shows it now.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LiveSeat {
    pub seat: i64,
    /// The triple count the leaderboard shows; None when the log never gave
    /// one for this hero (no triple yet, as far as the log says).
    pub triples: Option<i64>,
    /// The newest trinket in each slot, in the tooltip's order.
    pub trinkets: Vec<LiveTrinket>,
}

/// Each of `seats` as the leaderboard shows it, in the same order. `None`
/// when the report has no leaderboard values at all (a record older than
/// parser revision 5): not recorded, never "none".
pub fn seats_shown(report: &Value, seats: &[i64]) -> Option<Vec<LiveSeat>> {
    let lobby = report.get("lobby")?.as_array()?;
    if lobby.is_empty() || lobby.iter().any(|p| p.get("triples").is_none()) {
        return None;
    }
    let seat_of = |pid: i64| {
        lobby
            .iter()
            .find(|p| p.get("player_id").and_then(Value::as_i64) == Some(pid))
    };
    Some(
        seats
            .iter()
            .map(|&seat| match seat_of(seat) {
                Some(p) => seat_shown(seat, p),
                None => LiveSeat {
                    seat,
                    triples: None,
                    trinkets: Vec::new(),
                },
            })
            .collect(),
    )
}

fn seat_shown(seat: i64, p: &Value) -> LiveSeat {
    let triples = list(p, "triples")
        .iter()
        .take(MAX_CHANGES)
        .filter_map(|t| t.get("count")?.as_i64())
        .filter(|c| *c >= 0)
        .last();
    let changes: Vec<(String, i64)> = list(p, "trinkets")
        .iter()
        .take(MAX_CHANGES)
        .filter_map(|t| {
            let slot = text(t, "slot").filter(|s| SLOTS.contains(&s.as_str()))?;
            let dbf = t
                .get("dbf_id")?
                .as_i64()
                .filter(|d| (1..=MAX_DBF_ID).contains(d))?;
            Some((slot, dbf))
        })
        .collect();
    let trinkets = SLOTS
        .iter()
        .filter_map(|slot| {
            let (_, dbf) = changes.iter().rev().find(|(s, _)| s == slot)?;
            Some(LiveTrinket {
                slot: (*slot).to_string(),
                dbf_id: *dbf,
                card_id: None,
            })
        })
        .collect();
    LiveSeat {
        seat,
        triples,
        trinkets,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn report() -> Value {
        json!({"lobby": [
            {"player_id": 1, "hero": "H_ME", "triples": [], "trinkets": []},
            {"player_id": 2, "hero": "H_2",
             "triples": [{"turn": 5, "count": 1}, {"turn": 8, "count": 3}],
             "trinkets": [
                {"turn": 9, "slot": "second", "dbf_id": 222},
                {"turn": 6, "slot": "first", "dbf_id": 111},
                {"turn": 12, "slot": "first", "dbf_id": 112},
             ]},
            {"player_id": 3, "hero": "H_3", "triples": [], "trinkets": []},
        ]})
    }

    #[test]
    fn the_newest_count_and_the_newest_trinket_of_each_slot_in_the_tooltips_order() {
        let shown = seats_shown(&report(), &[2]).expect("recorded");
        assert_eq!(shown[0].triples, Some(3));
        let trinkets: Vec<_> = shown[0]
            .trinkets
            .iter()
            .map(|t| (t.slot.as_str(), t.dbf_id))
            .collect();
        assert_eq!(trinkets, [("first", 112), ("second", 222)], "an upgrade replaces the first");
        assert!(shown[0].trinkets.iter().all(|t| t.card_id.is_none()));
    }

    #[test]
    fn a_hero_with_no_triple_in_the_log_has_no_count_not_zero() {
        let shown = seats_shown(&report(), &[3]).expect("recorded");
        assert_eq!(shown[0].triples, None);
        assert!(shown[0].trinkets.is_empty());
    }

    #[test]
    fn a_record_older_than_revision_5_is_not_recorded() {
        let old = json!({"lobby": [{"player_id": 2, "hero": "H_2"}]});
        assert_eq!(seats_shown(&old, &[2]), None);
        assert_eq!(seats_shown(&json!({}), &[2]), None);
    }

    #[test]
    fn bad_slots_and_ids_are_dropped() {
        let mut r = report();
        r["lobby"][1]["trinkets"] = json!([
            {"turn": 6, "slot": "third", "dbf_id": 5},
            {"turn": 6, "slot": "first", "dbf_id": 0},
            {"turn": 6, "slot": "first", "dbf_id": "7"},
            {"turn": 6, "slot": "hero_power", "dbf_id": 9},
        ]);
        let shown = seats_shown(&r, &[2]).expect("recorded");
        let trinkets: Vec<_> = shown[0]
            .trinkets
            .iter()
            .map(|t| (t.slot.as_str(), t.dbf_id))
            .collect();
        assert_eq!(trinkets, [("hero_power", 9)]);
    }

    #[test]
    fn seats_keep_the_order_asked_for() {
        let shown = seats_shown(&report(), &[3, 2]).expect("recorded");
        let seats: Vec<_> = shown.iter().map(|s| s.seat).collect();
        assert_eq!(seats, [3, 2]);
    }
}
