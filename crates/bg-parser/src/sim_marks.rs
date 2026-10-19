//! Marks a hero power leaves on a board minion that the combat simulator
//! reads (T-631): the enchantment that gives the minion an effect for the
//! rest of the combat, by its exact card id. Fragrant Phylactery's start of
//! combat gives the lowest-Attack minion "Deathrattle: give your other
//! minions this minion's stats" as enchantment `BG20_HERO_282e2` (the user's
//! logs: on the chosen minion in 4 of 4 combats, removed after the combat),
//! so a board taken at the first attack carries it.
//!
//! The game shows the enchantment on the minion, as any buff (D-099,
//! D-142, D-163). Rust-only: never saved or uploaded, like
//! [`crate::report::SIM_KEYWORDS`].

use std::collections::HashMap;

use crate::state::{Board, Entity};

/// The marks read, by enchantment card id.
pub const SIM_MARKS: [&str; 1] = ["BG20_HERO_282e2"];

/// Minion entity id -> the marks on it, each once, in [`SIM_MARKS`] order.
pub type MarkIndex = HashMap<i64, Vec<&'static str>>;

pub fn mark_index(board: &Board) -> MarkIndex {
    let mut index = MarkIndex::new();
    for (minion, mark) in board.iter().filter_map(attached_mark) {
        let marks = index.entry(minion).or_default();
        if !marks.contains(&mark) {
            marks.push(mark);
        }
    }
    for marks in index.values_mut() {
        marks.sort_by_key(|m| SIM_MARKS.iter().position(|s| s == m));
    }
    index
}

fn attached_mark(e: &Entity) -> Option<(i64, &'static str)> {
    if !e.is("CARDTYPE", "ENCHANTMENT") || !e.is("ZONE", "PLAY") {
        return None;
    }
    let card = e.card_id.as_deref()?;
    let mark = SIM_MARKS.iter().find(|m| **m == card)?;
    Some((e.opt_int("ATTACHED")?, mark))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::power::Value;

    fn entity(id: i64, card: &str, tags: &[(&str, &str)]) -> Entity {
        let tags = tags
            .iter()
            .map(|(t, v)| {
                let value = v
                    .parse()
                    .map(Value::Int)
                    .unwrap_or(Value::Name(v.to_string()));
                (t.to_string(), value)
            })
            .collect();
        Entity {
            id,
            card_id: Some(card.into()),
            tags,
        }
    }

    fn enchantment(id: i64, card: &str, on: &str, zone: &str) -> Entity {
        entity(
            id,
            card,
            &[
                ("CARDTYPE", "ENCHANTMENT"),
                ("ZONE", zone),
                ("ATTACHED", on),
            ],
        )
    }

    fn board(entities: Vec<Entity>) -> Board {
        let mut board = Board::default();
        for e in entities {
            board.create(e).unwrap();
        }
        board
    }

    #[test]
    fn the_phylactery_mark_in_play_is_read_on_its_minion_once() {
        let b = board(vec![
            enchantment(10, "BG20_HERO_282e2", "5", "PLAY"),
            enchantment(11, "BG20_HERO_282e2", "5", "PLAY"),
            // Its stats given to another minion, a buff from another card, a
            // mark already removed, and a minion that is no enchantment.
            enchantment(12, "BG20_HERO_282pe", "6", "PLAY"),
            enchantment(13, "BG20_GEMe", "6", "PLAY"),
            enchantment(14, "BG20_HERO_282e2", "7", "REMOVEDFROMGAME"),
            entity(
                15,
                "BG20_HERO_282e2",
                &[("CARDTYPE", "MINION"), ("ZONE", "PLAY")],
            ),
        ]);
        let index = mark_index(&b);
        assert_eq!(index.get(&5), Some(&vec!["BG20_HERO_282e2"]));
        assert_eq!(index.len(), 1);
    }

    #[test]
    fn a_mark_with_no_minion_is_left_out() {
        let b = board(vec![entity(
            10,
            "BG20_HERO_282e2",
            &[("CARDTYPE", "ENCHANTMENT"), ("ZONE", "PLAY")],
        )]);
        assert!(mark_index(&b).is_empty());
    }
}
