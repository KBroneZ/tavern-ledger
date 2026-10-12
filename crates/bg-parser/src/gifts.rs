//! Dark Gifts on board minions (T-528, D-107). On the board a gift is an
//! enchantment in `PLAY` attached to the minion (`ATTACHED` = the minion's
//! id); the minion also has `HAS_DARK_GIFT` 1. Combat copies carry both, for
//! the player's and the opponent's minions alike, and the game shows the
//! gift on the minion (design notes).
//!
//! A gift is saved as the gift spell's card id (`BG36_MidGameEffect_000tNN`,
//! the id the offers name, D-058, D-100), not the enchantment's: the
//! enchantment ids differ per gift, and some gifts also attach hidden "stat
//! holders" that do not mean the minion carries a gift (the Golemancy Golem
//! has one). Only the enchantments in [`GIFT_ENCHANTMENTS`] count.

use std::collections::HashMap;

use crate::state::{Board, Entity};

/// The most gifts kept on one minion: far more than the two the user's logs
/// show, so a strange log cannot grow a board.
pub const MAX_GIFTS: usize = 4;

const PREFIX: &str = "BG36_MidGameEffect_";

/// Enchantment card id (after [`PREFIX`], or whole when it has another
/// prefix) -> gift id (after [`PREFIX`]). Every season 36 gift (47) has one
/// or two; holders are not listed.
const GIFT_ENCHANTMENTS: &[(&str, &str)] = &[
    ("000te2", "000t"),
    ("000t2e2", "000t2"),
    ("000t3e2", "000t3"),
    ("000t4e", "000t4"),
    ("000t4e2", "000t4"),
    ("000t5e2", "000t5"),
    ("000t7e2", "000t7"),
    ("000t9e2", "000t9"),
    ("000t10e2", "000t10"),
    ("000t11e", "000t11"),
    ("000t11e2", "000t11"),
    ("000t12e", "000t12"),
    // Harpy's Talons: the logs attach a card from another set.
    ("EDR_100t13e", "000t13"),
    ("000t13e", "000t13"),
    ("000t14e", "000t14"),
    ("000t15e", "000t15"),
    ("000t16e", "000t16"),
    ("000t18e", "000t18"),
    ("000t21e", "000t21"),
    ("000t22e", "000t22"),
    ("000t28e", "000t28"),
    ("000t28te", "000t28t"),
    ("000t29e", "000t29"),
    ("000t29te", "000t29t"),
    ("000t30e", "000t30"),
    ("000t30te", "000t30t"),
    ("000t50e", "000t50"),
    ("000t51e", "000t51"),
    ("000t52e", "000t52"),
    ("000t60e", "000t60"),
    ("000t61e", "000t61"),
    ("000t62e", "000t62"),
    ("000t64e", "000t64"),
    ("000t64te", "000t64t"),
    ("000t65e", "000t65"),
    ("000t66e", "000t66"),
    ("000t69e", "000t69"),
    ("000t71e", "000t71"),
    ("000t72e", "000t72"),
    ("000t73e", "000t73"),
    ("000t74e", "000t74"),
    ("000t75e", "000t75"),
    ("000t78e", "000t78"),
    ("000t79e", "000t79"),
    ("000t80e", "000t80"),
    ("000t81e", "000t81"),
    ("000t82e", "000t82"),
    ("000t88e", "000t88"),
    ("000t89e", "000t89"),
    ("000t90e", "000t90"),
];

/// Every gift id the table maps to, whole (`BG36_MidGameEffect_000tNN`).
pub const GIFT_IDS: [&str; 47] = [
    "BG36_MidGameEffect_000t",
    "BG36_MidGameEffect_000t2",
    "BG36_MidGameEffect_000t3",
    "BG36_MidGameEffect_000t4",
    "BG36_MidGameEffect_000t5",
    "BG36_MidGameEffect_000t7",
    "BG36_MidGameEffect_000t9",
    "BG36_MidGameEffect_000t10",
    "BG36_MidGameEffect_000t11",
    "BG36_MidGameEffect_000t12",
    "BG36_MidGameEffect_000t13",
    "BG36_MidGameEffect_000t14",
    "BG36_MidGameEffect_000t15",
    "BG36_MidGameEffect_000t16",
    "BG36_MidGameEffect_000t18",
    "BG36_MidGameEffect_000t21",
    "BG36_MidGameEffect_000t22",
    "BG36_MidGameEffect_000t28",
    "BG36_MidGameEffect_000t28t",
    "BG36_MidGameEffect_000t29",
    "BG36_MidGameEffect_000t29t",
    "BG36_MidGameEffect_000t30",
    "BG36_MidGameEffect_000t30t",
    "BG36_MidGameEffect_000t50",
    "BG36_MidGameEffect_000t51",
    "BG36_MidGameEffect_000t52",
    "BG36_MidGameEffect_000t60",
    "BG36_MidGameEffect_000t61",
    "BG36_MidGameEffect_000t62",
    "BG36_MidGameEffect_000t64",
    "BG36_MidGameEffect_000t64t",
    "BG36_MidGameEffect_000t65",
    "BG36_MidGameEffect_000t66",
    "BG36_MidGameEffect_000t69",
    "BG36_MidGameEffect_000t71",
    "BG36_MidGameEffect_000t72",
    "BG36_MidGameEffect_000t73",
    "BG36_MidGameEffect_000t74",
    "BG36_MidGameEffect_000t75",
    "BG36_MidGameEffect_000t78",
    "BG36_MidGameEffect_000t79",
    "BG36_MidGameEffect_000t80",
    "BG36_MidGameEffect_000t81",
    "BG36_MidGameEffect_000t82",
    "BG36_MidGameEffect_000t88",
    "BG36_MidGameEffect_000t89",
    "BG36_MidGameEffect_000t90",
];

/// The gift an enchantment card id means, whole (`BG36_MidGameEffect_000tNN`);
/// `None` for a stat holder or any other enchantment.
pub fn gift_of_enchantment(card_id: &str) -> Option<&'static str> {
    let short = card_id.strip_prefix(PREFIX).unwrap_or(card_id);
    if short == card_id && !card_id.starts_with("EDR_") {
        return None;
    }
    let (_, gift) = GIFT_ENCHANTMENTS.iter().find(|(e, _)| *e == short)?;
    GIFT_IDS
        .iter()
        .copied()
        .find(|id| id.strip_prefix(PREFIX) == Some(*gift))
}

/// Minion entity id -> its gifts, in the order the enchantments were
/// created, each once, at most [`MAX_GIFTS`]. One pass over the board.
pub type GiftIndex = HashMap<i64, Vec<&'static str>>;

pub fn gift_index(board: &Board) -> GiftIndex {
    let mut index = GiftIndex::new();
    for e in board.iter() {
        if let Some((minion, gift)) = attached_gift(e) {
            add(index.entry(minion).or_default(), gift);
        }
    }
    index
}

/// The gifts on one minion; a pass over the board, for a single minion.
pub fn gifts_on(board: &Board, minion: i64) -> Vec<&'static str> {
    let mut gifts = Vec::new();
    for (_, gift) in board
        .iter()
        .filter_map(attached_gift)
        .filter(|(m, _)| *m == minion)
    {
        add(&mut gifts, gift);
    }
    gifts
}

/// True when the game marks the minion as gifted (`HAS_DARK_GIFT`) but no
/// gift enchantment says which: shown as "Dark Gift (unknown)", never passed
/// to the simulator as a gift (2 of 121 gifted minions on the user's logs).
pub fn gift_unknown(e: &Entity, gifts: &[&str]) -> bool {
    gifts.is_empty() && e.int("HAS_DARK_GIFT") > 0
}

fn attached_gift(e: &Entity) -> Option<(i64, &'static str)> {
    if !e.is("CARDTYPE", "ENCHANTMENT") || !e.is("ZONE", "PLAY") {
        return None;
    }
    let gift = gift_of_enchantment(e.card_id.as_deref()?)?;
    Some((e.opt_int("ATTACHED")?, gift))
}

fn add(gifts: &mut Vec<&'static str>, gift: &'static str) {
    if gifts.len() < MAX_GIFTS && !gifts.contains(&gift) {
        gifts.push(gift);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_gift_has_an_enchantment_and_every_enchantment_a_gift() {
        for id in GIFT_IDS {
            let short = id.strip_prefix(PREFIX).unwrap();
            assert!(
                GIFT_ENCHANTMENTS.iter().any(|(_, g)| *g == short),
                "{id} has no enchantment"
            );
        }
        for (enchantment, gift) in GIFT_ENCHANTMENTS {
            assert!(
                GIFT_IDS.contains(&format!("{PREFIX}{gift}").as_str()),
                "{enchantment} maps to an unknown gift"
            );
        }
    }

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

    fn enchantment(id: i64, card: &str, on: i64, zone: &str) -> Entity {
        let on = on.to_string();
        entity(
            id,
            card,
            &[
                ("CARDTYPE", "ENCHANTMENT"),
                ("ZONE", zone),
                ("ATTACHED", &on),
            ],
        )
    }

    fn board(entities: Vec<Entity>) -> Board {
        let mut board = Board::default();
        for e in entities {
            board.create(e);
        }
        board
    }

    #[test]
    fn gifts_are_the_text_enchantments_in_play_on_the_minion() {
        let b = board(vec![
            entity(
                10,
                "BG25_008",
                &[("CARDTYPE", "MINION"), ("HAS_DARK_GIFT", "1")],
            ),
            entity(11, "BG29_864t", &[("CARDTYPE", "MINION")]),
            enchantment(20, "BG36_MidGameEffect_000t9e2", 10, "PLAY"),
            // The holder that keeps the numbers, and a copy of the same gift.
            enchantment(21, "BG36_MidGameEffect_000t9e", 10, "PLAY"),
            enchantment(22, "BG36_MidGameEffect_000t9e2", 10, "PLAY"),
            enchantment(23, "EDR_100t13e", 10, "PLAY"),
            // A gift that left the minion.
            enchantment(24, "BG36_MidGameEffect_000t90e", 10, "GRAVEYARD"),
            // The Golemancy Golem: stats set, no gift.
            enchantment(25, "BG36_MidGameEffect_000t61e2", 11, "PLAY"),
        ]);
        let index = gift_index(&b);
        let both = vec!["BG36_MidGameEffect_000t9", "BG36_MidGameEffect_000t13"];
        assert_eq!(index.get(&10), Some(&both));
        assert_eq!(index.get(&11), None);
        assert_eq!(gifts_on(&b, 10), both);
        assert!(gifts_on(&b, 11).is_empty());
    }

    #[test]
    fn a_minion_keeps_at_most_four_gifts() {
        let mut entities = vec![entity(10, "BG25_008", &[("CARDTYPE", "MINION")])];
        for (i, card) in ["000t9e2", "000t13e", "000t16e", "000t90e", "000t71e"]
            .iter()
            .enumerate()
        {
            entities.push(enchantment(
                20 + i as i64,
                &format!("{PREFIX}{card}"),
                10,
                "PLAY",
            ));
        }
        assert_eq!(gifts_on(&board(entities), 10).len(), MAX_GIFTS);
    }

    #[test]
    fn marked_gifted_with_no_gift_enchantment_is_unknown() {
        let marked = entity(10, "BG25_008", &[("HAS_DARK_GIFT", "1")]);
        let plain = entity(11, "BG25_008", &[]);
        assert!(gift_unknown(&marked, &[]));
        assert!(!gift_unknown(&marked, &["BG36_MidGameEffect_000t9"]));
        assert!(!gift_unknown(&plain, &[]));
    }

    #[test]
    fn text_enchantments_map_to_their_gift_and_holders_to_none() {
        let cases = [
            ("BG36_MidGameEffect_000te2", Some("BG36_MidGameEffect_000t")),
            (
                "BG36_MidGameEffect_000t9e2",
                Some("BG36_MidGameEffect_000t9"),
            ),
            (
                "BG36_MidGameEffect_000t90e",
                Some("BG36_MidGameEffect_000t90"),
            ),
            (
                "BG36_MidGameEffect_000t64te",
                Some("BG36_MidGameEffect_000t64t"),
            ),
            ("EDR_100t13e", Some("BG36_MidGameEffect_000t13")),
            (
                "BG36_MidGameEffect_000t4e",
                Some("BG36_MidGameEffect_000t4"),
            ),
            // Stat holders and the Golemancy Golem's stats.
            ("BG36_MidGameEffect_000te", None),
            ("BG36_MidGameEffect_000t9e", None),
            ("BG36_MidGameEffect_000t4e4", None),
            ("BG36_MidGameEffect_000t61e2", None),
            ("BG36_MidGameEffect_000t64te2", None),
            // The gift spell itself and other cards.
            ("BG36_MidGameEffect_000t90", None),
            ("000t90e", None),
            ("EDR_100", None),
            ("BG25_008e", None),
        ];
        for (card, gift) in cases {
            assert_eq!(gift_of_enchantment(card), gift, "{card}");
        }
    }
}
