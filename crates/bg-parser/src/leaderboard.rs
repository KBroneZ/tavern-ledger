//! Each lobby hero's triples and trinkets as the game's leaderboard tooltip
//! shows them (T-207, D-058). Rust-only: the Python prototype does not read
//! them.
//!
//! What the log gives (checked on 11 real games, docs/research/parser-round-2.md):
//! - `PLAYER_TRIPLES` on each leaderboard hero is its triple count; on the
//!   local hero it equals the triple rewards the player got in Solo;
//! - `BACON_FIRST_TRINKET_DATABASE_ID` and `BACON_SECOND_TRINKET_DATABASE_ID`
//!   on each leaderboard hero are the database ids of its trinkets; on the
//!   local hero they equal the trinket the player picked every time (21 of
//!   21). A trinket that changes (an upgrade) sets the tag again.
//!   `BACON_HEROPOWER_TRINKET_DATABASE_ID` is a trinket that became a hero
//!   power.
//!
//! Only the values on leaderboard heroes are kept, never those on the copies
//! the game makes for a combat, and only when they change; from a tag change
//! or from the packet that creates or shows the hero. A trinket tag set back
//! to 0 is not kept: the list says what the leaderboard showed, not when a
//! trinket left it (never seen in the user's logs).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::collector::MAX_DBF_ID;

/// Changes kept per hero and list, far more than a game has.
pub const MAX_CHANGES: usize = 64;
/// No hero makes this many triples.
pub const MAX_TRIPLES: i64 = 1_000;

/// The shop record's bound on turns (shop.rs), which upload-game checks too.
pub const MAX_TURN: i64 = crate::shop::MAX_TURNS as i64;

pub const TRIPLES: &str = "PLAYER_TRIPLES";
const TRINKET_TAGS: [(&str, TrinketSlot); 3] = [
    ("BACON_FIRST_TRINKET_DATABASE_ID", TrinketSlot::First),
    ("BACON_SECOND_TRINKET_DATABASE_ID", TrinketSlot::Second),
    (
        "BACON_HEROPOWER_TRINKET_DATABASE_ID",
        TrinketSlot::HeroPower,
    ),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrinketSlot {
    First,
    Second,
    /// A trinket that became a hero power.
    HeroPower,
}

/// The triple count the leaderboard shows from this turn on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TriplesSeen {
    /// The shop turn (a combat counts as its shop turn's).
    pub turn: i64,
    pub count: i64,
}

/// A trinket the leaderboard shows from this turn on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrinketSeen {
    pub turn: i64,
    pub slot: TrinketSlot,
    /// Database id of the trinket card (turned into a card id with the card
    /// data, as the skin links are).
    pub dbf_id: i64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HeroLog {
    pub triples: Vec<TriplesSeen>,
    pub trinkets: Vec<TrinketSeen>,
}

/// What the leaderboard showed for each lobby hero, by player id.
#[derive(Clone, Debug, Default)]
pub struct LeaderboardTracker {
    heroes: BTreeMap<i64, HeroLog>,
}

/// True for the tags this tracker reads.
pub fn is_leaderboard_tag(tag: &str) -> bool {
    tag == TRIPLES || TRINKET_TAGS.iter().any(|(t, _)| *t == tag)
}

impl LeaderboardTracker {
    /// `tag` changed to `value` on the leaderboard hero of `player_id`
    /// during game `TURN` `game_turn`.
    pub fn on_tag(&mut self, player_id: i64, tag: &str, value: i64, game_turn: i64) {
        if !(1..=64).contains(&player_id) {
            return;
        }
        let turn = shop_turn(game_turn);
        // Past the shop record's bound: not a turn the game has.
        if turn > MAX_TURN {
            return;
        }
        let log = self.heroes.entry(player_id).or_default();
        if tag == TRIPLES {
            let same = log.triples.last().is_some_and(|t| t.count == value);
            let known = (0..=MAX_TRIPLES).contains(&value);
            if known && !same && log.triples.len() < MAX_CHANGES {
                log.triples.push(TriplesSeen { turn, count: value });
            }
            return;
        }
        let Some((_, slot)) = TRINKET_TAGS.iter().find(|(t, _)| *t == tag) else {
            return;
        };
        let same = log
            .trinkets
            .iter()
            .rev()
            .find(|t| t.slot == *slot)
            .is_some_and(|t| t.dbf_id == value);
        let known = (1..=MAX_DBF_ID).contains(&value);
        if known && !same && log.trinkets.len() < MAX_CHANGES {
            log.trinkets.push(TrinketSeen {
                turn,
                slot: *slot,
                dbf_id: value,
            });
        }
    }

    pub fn of(&self, player_id: i64) -> HeroLog {
        self.heroes.get(&player_id).cloned().unwrap_or_default()
    }
}

/// `TURN` 1 is shop turn 1 and its combat (`TURN` 2) stays turn 1, as in the
/// shop record; hero select is turn 0.
pub fn shop_turn(game_turn: i64) -> i64 {
    game_turn.saturating_add(1).div_euclid(2).max(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn triples_are_kept_when_they_change() {
        let mut t = LeaderboardTracker::default();
        t.on_tag(3, TRIPLES, 1, 9);
        t.on_tag(3, TRIPLES, 1, 10); // a combat copy of the same value
        t.on_tag(3, TRIPLES, 2, 13);
        let log = t.of(3);
        assert_eq!(
            log.triples,
            [
                TriplesSeen { turn: 5, count: 1 },
                TriplesSeen { turn: 7, count: 2 }
            ]
        );
        assert!(log.trinkets.is_empty());
        assert_eq!(t.of(4), HeroLog::default());
    }

    #[test]
    fn trinkets_are_kept_per_slot_with_upgrades() {
        let mut t = LeaderboardTracker::default();
        t.on_tag(2, "BACON_FIRST_TRINKET_DATABASE_ID", 111, 11);
        t.on_tag(2, "BACON_SECOND_TRINKET_DATABASE_ID", 222, 17);
        t.on_tag(2, "BACON_FIRST_TRINKET_DATABASE_ID", 111, 17);
        t.on_tag(2, "BACON_FIRST_TRINKET_DATABASE_ID", 333, 19);
        let slots: Vec<_> = t
            .of(2)
            .trinkets
            .iter()
            .map(|s| (s.turn, s.slot, s.dbf_id))
            .collect();
        assert_eq!(
            slots,
            [
                (6, TrinketSlot::First, 111),
                (9, TrinketSlot::Second, 222),
                (10, TrinketSlot::First, 333)
            ]
        );
    }

    #[test]
    fn strange_values_are_left_out_and_lists_are_bounded() {
        let mut t = LeaderboardTracker::default();
        t.on_tag(0, TRIPLES, 1, 3);
        t.on_tag(65, TRIPLES, 1, 3);
        t.on_tag(1, TRIPLES, -1, 3);
        t.on_tag(1, "BACON_FIRST_TRINKET_DATABASE_ID", 0, 3);
        t.on_tag(1, "BACON_FIRST_TRINKET_DATABASE_ID", MAX_DBF_ID + 1, 3);
        assert_eq!(t.of(1), HeroLog::default());
        for n in 0..200 {
            t.on_tag(1, TRIPLES, n, 3);
        }
        assert_eq!(t.of(1).triples.len(), MAX_CHANGES);
    }

    #[test]
    fn shop_turns_follow_the_shop_record() {
        assert_eq!([0, 1, 2, 3, 4, -5].map(shop_turn), [0, 1, 1, 2, 2, 0]);
    }
}
