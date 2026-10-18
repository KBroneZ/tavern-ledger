//! Each side's hero powers and trinkets as a combat leg starts (T-628):
//! what the simulator needs beside the board. Rust-only: not saved or
//! uploaded (the saved report does not carry them yet).
//!
//! In the log both are entities in play controlled by the side's player:
//! `CARDTYPE=HERO_POWER` and `CARDTYPE=BATTLEGROUND_TRINKET`. The opponent's
//! are the copies the game shows when it sets the combat up. The numbers a
//! card's text shows are its `TAG_SCRIPT_DATA_NUM_1..5`; a trinket's slot is
//! its `TAG_SCRIPT_DATA_NUM_6` (1 lesser, 2 greater, 3 a hero power that
//! became a trinket), a second hero power has `ADDITIONAL_HERO_POWER_INDEX`
//! 1, and a minion a hero power names is its `TAG_SCRIPT_DATA_ENT_1`.

use crate::replay::board_entities;
use crate::state::{Board, Entity};

/// Kept per kind and side: more than any board has (two hero powers, three
/// trinket slots), so a strange log cannot grow the record.
pub const MAX_KIT_CARDS: usize = 4;

const COUNTER_TAGS: [&str; 5] = [
    "TAG_SCRIPT_DATA_NUM_1",
    "TAG_SCRIPT_DATA_NUM_2",
    "TAG_SCRIPT_DATA_NUM_3",
    "TAG_SCRIPT_DATA_NUM_4",
    "TAG_SCRIPT_DATA_NUM_5",
];

/// A hero power or trinket in play.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KitCard {
    pub card_id: String,
    /// Hero powers: 0 the hero's own, 1 a second one. Trinkets: 1 lesser,
    /// 2 greater, 3 a hero power that became a trinket; 0 when the log
    /// gives none.
    pub slot: i64,
    /// The numbers its text shows (`TAG_SCRIPT_DATA_NUM_1..5`); 0 where the
    /// log gives none, as the game prints no zero.
    pub counters: [i64; 5],
    /// The board position of the friendly minion it names, when that
    /// minion is on its side's board.
    pub target: Option<i64>,
}

/// One side's hero powers and trinkets, each in slot order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HeroKit {
    pub powers: Vec<KitCard>,
    pub trinkets: Vec<KitCard>,
}

/// The hero powers and trinkets `controller` has in play now.
pub fn kit_of(board: &Board, controller: i64) -> HeroKit {
    let minions = board_entities(board, controller);
    let card = |e: &Entity, slot_tag: &str| -> Option<KitCard> {
        let card_id = e.card_id.clone()?;
        let target = e
            .opt_int("TAG_SCRIPT_DATA_ENT_1")
            .and_then(|id| minions.iter().find(|m| m.id == id))
            .map(|m| m.int("ZONE_POSITION"));
        Some(KitCard {
            card_id,
            slot: e.int(slot_tag),
            counters: COUNTER_TAGS.map(|t| e.int(t)),
            target,
        })
    };
    let of_type = |card_type: &str, slot_tag: &str| -> Vec<KitCard> {
        let mut out: Vec<(i64, KitCard)> = board
            .iter()
            .filter(|e| {
                e.is("CARDTYPE", card_type)
                    && e.is("ZONE", "PLAY")
                    && e.int("CONTROLLER") == controller
            })
            .filter_map(|e| card(e, slot_tag).map(|c| (e.id, c)))
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
