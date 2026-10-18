//! Each side's hero powers and trinkets as a combat leg starts (T-628):
//! what the simulator needs beside the board. Saved with each board that
//! fought since parser revision 16 (both sides, the opponent's without a
//! held minion; see [`crate::report::SAVE_OPPONENT_KIT`]); a record without
//! the key did not read them ("not recorded").
//!
//! In the log both are entities in play controlled by the side's player:
//! `CARDTYPE=HERO_POWER` and `CARDTYPE=BATTLEGROUND_TRINKET`. The opponent's
//! are the copies the game shows when it sets the combat up. The numbers a
//! card's text shows are its `TAG_SCRIPT_DATA_NUM_1..5`; a trinket's slot is
//! its `TAG_SCRIPT_DATA_NUM_6` (1 lesser, 2 greater, 3 a hero power that
//! became a trinket), a second hero power has `ADDITIONAL_HERO_POWER_INDEX`
//! 1, and a minion a hero power names is its `TAG_SCRIPT_DATA_ENT_1`.
//!
//! A friendly minion a hero power names (Rapid Reanimation, Glorious Gloop)
//! carries an enchantment of that power, its card id followed by `e`
//! (`BG25_HERO_103pe`, `BGDUO_HERO_101pe2`): on a Duos teammate's board the
//! minions are copies and `TAG_SCRIPT_DATA_ENT_1` names the original, which
//! the log never shows, so the mark is how the minion is found (6 of 6 such
//! combats in the user's logs). A minion a hero power holds off the board
//! (Lock and Load) is the entity its `TAG_SCRIPT_DATA_ENT_1` names, kept
//! aside with its card id, stats and buffs.

use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};

use crate::buffs::buffs_on;
use crate::gifts::gifts_on;
use crate::replay::{board_entities, minion_of};
use crate::report::Minion;
use crate::state::{Board, Entity};

/// Kept per kind and side: more than any board has (two hero powers, three
/// trinket slots), so a strange log cannot grow the record.
pub const MAX_KIT_CARDS: usize = 4;
/// A number a card's text shows above this, or below 0, is not kept (0).
pub const MAX_KIT_NUMBER: i64 = i32::MAX as i64;
/// The highest slot a hero power or trinket has (a trinket's hero-power
/// slot); a slot outside 0 to this is kept as 0 ("none given").
pub const MAX_KIT_SLOT: i64 = 3;
/// The board positions a named minion can have; outside them it is not
/// kept (no target).
const BOARD_POSITIONS: std::ops::RangeInclusive<i64> = 1..=7;

const COUNTER_TAGS: [&str; 5] = [
    "TAG_SCRIPT_DATA_NUM_1",
    "TAG_SCRIPT_DATA_NUM_2",
    "TAG_SCRIPT_DATA_NUM_3",
    "TAG_SCRIPT_DATA_NUM_4",
    "TAG_SCRIPT_DATA_NUM_5",
];

/// A hero power or trinket in play.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct KitCard {
    pub card_id: String,
    /// Hero powers: 0 the hero's own, 1 a second one. Trinkets: 1 lesser,
    /// 2 greater, 3 a hero power that became a trinket; 0 when the log
    /// gives none.
    pub slot: i64,
    /// The numbers its text shows (`TAG_SCRIPT_DATA_NUM_1..5`); 0 where the
    /// log gives none, as the game prints no zero. Left out of the JSON when
    /// all are 0.
    #[serde(skip_serializing_if = "all_zero")]
    pub counters: [i64; 5],
    /// The board position of the friendly minion it names, when that
    /// minion is on its side's board.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<i64>,
    /// The minion it holds off the board (Lock and Load): the entity it
    /// names when that is a minion with a card id outside the play zone.
    /// Saved with its card id, stats, golden flag, keywords and Dark Gifts.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "held_json")]
    pub held: Option<Minion>,
}

fn all_zero(n: &[i64; 5]) -> bool {
    n.iter().all(|v| *v == 0)
}

fn held_json<S: Serializer>(held: &Option<Minion>, s: S) -> Result<S::Ok, S::Error> {
    let Some(m) = held else {
        return s.serialize_none();
    };
    let mut map = s.serialize_map(None)?;
    map.serialize_entry("card_id", &m.card_id)?;
    map.serialize_entry("atk", &m.atk)?;
    map.serialize_entry("health", &m.health)?;
    map.serialize_entry("golden", &m.golden)?;
    map.serialize_entry("keywords", &m.keywords)?;
    if !m.gifts.is_empty() {
        map.serialize_entry("gifts", &m.gifts)?;
    }
    map.end()
}

/// One side's hero powers and trinkets, each in slot order.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct HeroKit {
    pub powers: Vec<KitCard>,
    pub trinkets: Vec<KitCard>,
}

/// The hero powers and trinkets `controller` has in play now.
pub fn kit_of(board: &Board, controller: i64) -> HeroKit {
    let minions = board_entities(board, controller);
    let position = |id: i64| {
        minions
            .iter()
            .find(|m| m.id == id)
            .map(|m| m.int("ZONE_POSITION"))
            .filter(|p| BOARD_POSITIONS.contains(p))
    };
    // Only a hero power names or holds a minion (Rapid Reanimation, Glorious
    // Gloop, Lock and Load); a trinket keeps its card, slot and numbers.
    let card = |e: &Entity, slot_tag: &str, power: bool| -> Option<KitCard> {
        let card_id = e.card_id.clone()?;
        let named = e.opt_int("TAG_SCRIPT_DATA_ENT_1").filter(|_| power);
        let target = named.and_then(position).or_else(|| {
            marked(board, &card_id, &minions)
                .filter(|_| power)
                .and_then(position)
        });
        Some(KitCard {
            held: named.and_then(|id| held(board, id)),
            card_id,
            slot: Some(e.int(slot_tag))
                .filter(|s| (0..=MAX_KIT_SLOT).contains(s))
                .unwrap_or(0),
            counters: COUNTER_TAGS.map(|t| e.int(t)).map(|n| {
                if (0..=MAX_KIT_NUMBER).contains(&n) {
                    n
                } else {
                    0
                }
            }),
            target,
        })
    };
    let of_type = |card_type: &str, slot_tag: &str| -> Vec<KitCard> {
        let power = card_type == "HERO_POWER";
        let mut out: Vec<(i64, KitCard)> = board
            .iter()
            .filter(|e| {
                e.is("CARDTYPE", card_type)
                    && e.is("ZONE", "PLAY")
                    && e.int("CONTROLLER") == controller
            })
            .filter_map(|e| card(e, slot_tag, power).map(|c| (e.id, c)))
            .collect();
        out.sort_by_key(|(id, c)| (c.slot, *id));
        out.truncate(MAX_KIT_CARDS);
        out.into_iter().map(|(_, c)| c).collect()
    };
    HeroKit {
        powers: of_type("HERO_POWER", "ADDITIONAL_HERO_POWER_INDEX"),
        trinkets: of_type("BATTLEGROUND_TRINKET", "TAG_SCRIPT_DATA_NUM_6"),
    }
}

/// The board minion carrying an enchantment of the card `card_id` (its id
/// followed by `e`), the left-most when several do.
fn marked(board: &Board, card_id: &str, minions: &[&Entity]) -> Option<i64> {
    let prefix = format!("{card_id}e");
    let marks: Vec<i64> = board
        .iter()
        .filter(|e| {
            e.is("CARDTYPE", "ENCHANTMENT")
                && e.is("ZONE", "PLAY")
                && e.card_id.as_deref().is_some_and(|c| c.starts_with(&prefix))
        })
        .map(|e| e.int("ATTACHED"))
        .collect();
    minions.iter().find(|m| marks.contains(&m.id)).map(|m| m.id)
}

/// The minion entity `id` when it is kept off the board with a card id.
fn held(board: &Board, id: i64) -> Option<Minion> {
    let e = board.get(id)?;
    let aside = e.is("CARDTYPE", "MINION") && e.card_id.is_some() && !e.is("ZONE", "PLAY");
    aside.then(|| minion_of(e, gifts_on(board, id), buffs_on(board, id)))
}
