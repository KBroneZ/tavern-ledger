//! Buffs on board minions (T-619, D-142): the enchantments the game shows on
//! a minion, grouped by the card that gave them and counted. On the board an
//! enchantment is an entity in `PLAY` attached to the minion (`ATTACHED` =
//! the minion's id); combat copies carry them too, for the player's and the
//! opponents' minions alike.
//!
//! What counts is what the game shows the player (D-004): enchantments with
//! `ENCHANTMENT_INVISIBLE` are the game's own holders and stay out, and so do
//! Dark Gift enchantments, which have their own marks ([`crate::gifts`]). The
//! log does not say how much attack and health each enchantment gave (the
//! numbers it carries are not those), so only the count is kept.

use std::collections::HashMap;

use serde::Serialize;

use crate::gifts::gift_of_enchantment;
use crate::state::{Board, Entity};

/// The most kinds of buff kept on one minion, so a strange log cannot grow a
/// board.
pub const MAX_BUFF_KINDS: usize = 12;
/// The most one kind is counted to: far more than a minion gets in a game.
pub const MAX_BUFF_COUNT: u32 = 999;

/// Dark Gift enchantments and their stat holders share this prefix.
const GIFT_PREFIX: &str = "BG36_MidGameEffect_";
/// The longest card id the upload accepts (`CARD_ID` in upload-game).
const MAX_ID: usize = 64;

/// One kind of buff on a minion: the card that gave it and how many of its
/// enchantments the minion has.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Buff {
    pub card_id: String,
    pub count: u32,
}

/// The card an enchantment comes from: its id without the enchantment suffix
/// (`BG20_GEMe2` is Blood Gem `BG20_GEM`, `BG31_880te2` the token
/// `BG31_880t`, `BG20_HERO_102pe` the hero `BG20_HERO_102`). An id with no
/// suffix is its own source. `None` for an id that is not plain letters,
/// digits and `_`, or has nothing left.
pub fn source_of_enchantment(card_id: &str) -> Option<String> {
    let plain = |c: char| c.is_ascii_alphanumeric() || c == '_';
    if card_id.is_empty() || card_id.len() > MAX_ID || !card_id.chars().all(plain) {
        return None;
    }
    let source = strip_suffix(card_id);
    (!source.is_empty()).then(|| source.to_string())
}

fn strip_suffix(id: &str) -> &str {
    if let Some(base) = id.strip_suffix("_Ench") {
        return base;
    }
    let rest = id.trim_end_matches(|c: char| c.is_ascii_digit());
    let Some(base) = rest.strip_suffix('E').or_else(|| lower_e_suffix(rest)) else {
        return id;
    };
    // The player's form of a card's enchantment (`pe`): the card is without it.
    let base = match base.strip_suffix('p') {
        Some(card) if card.ends_with(|c: char| c.is_ascii_digit()) => card,
        _ => base,
    };
    base.trim_end_matches('_')
}

/// A lower-case `e` ends an enchantment id, but not a word: `GEMe`, `011e`,
/// `880te` and `_e` do, `Consumed` does not.
fn lower_e_suffix(rest: &str) -> Option<&str> {
    let base = rest.strip_suffix('e')?;
    // An id that is only the letter has no card left.
    let Some(before) = base.chars().next_back() else {
        return Some(base);
    };
    (!before.is_ascii_lowercase() || before == 't' || before == 'p').then_some(base)
}

/// Minion entity id -> its buffs, in the order the enchantments were
/// created, each kind once with its count, at most [`MAX_BUFF_KINDS`]. One
/// pass over the board.
pub type BuffIndex = HashMap<i64, Vec<Buff>>;

pub fn buff_index(board: &Board) -> BuffIndex {
    let mut index = BuffIndex::new();
    for e in board.iter() {
        if let Some((minion, source)) = attached_buff(e) {
            add(index.entry(minion).or_default(), source);
        }
    }
    index
}

/// The buffs on one minion; a pass over the board, for a single minion.
pub fn buffs_on(board: &Board, minion: i64) -> Vec<Buff> {
    let mut buffs = Vec::new();
    for (_, source) in board
        .iter()
        .filter_map(attached_buff)
        .filter(|(m, _)| *m == minion)
    {
        add(&mut buffs, source);
    }
    buffs
}

fn attached_buff(e: &Entity) -> Option<(i64, String)> {
    if !e.is("CARDTYPE", "ENCHANTMENT")
        || !e.is("ZONE", "PLAY")
        || e.int("ENCHANTMENT_INVISIBLE") > 0
    {
        return None;
    }
    let card = e.card_id.as_deref()?;
    if card.starts_with(GIFT_PREFIX) || gift_of_enchantment(card).is_some() {
        return None;
    }
    Some((e.opt_int("ATTACHED")?, source_of_enchantment(card)?))
}

fn add(buffs: &mut Vec<Buff>, source: String) {
    if let Some(b) = buffs.iter_mut().find(|b| b.card_id == source) {
        b.count = (b.count + 1).min(MAX_BUFF_COUNT);
    } else if buffs.len() < MAX_BUFF_KINDS {
        buffs.push(Buff {
            card_id: source,
            count: 1,
        });
    }
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

    fn enchantment(id: i64, card: &str, on: i64, extra: &[(&str, &str)]) -> Entity {
        let on = on.to_string();
        let mut tags = vec![
            ("CARDTYPE", "ENCHANTMENT"),
            ("ZONE", "PLAY"),
            ("ATTACHED", on.as_str()),
        ];
        tags.extend_from_slice(extra);
        entity(id, card, &tags)
    }

    fn board(entities: Vec<Entity>) -> Board {
        let mut board = Board::default();
        for e in entities {
            board.create(e).unwrap();
        }
        board
    }

    fn buff(card: &str, count: u32) -> Buff {
        Buff {
            card_id: card.into(),
            count,
        }
    }

    #[test]
    fn the_source_card_is_the_enchantment_without_its_suffix() {
        let cases = [
            ("BG20_GEMe", Some("BG20_GEM")),
            ("BG20_GEMe2", Some("BG20_GEM")),
            ("BG25_011e2", Some("BG25_011")),
            ("BG31_880te2", Some("BG31_880t")),
            ("BG36_301te", Some("BG36_301t")),
            ("BG20_HERO_102pe", Some("BG20_HERO_102")),
            ("BG24_HERO_204pe2", Some("BG24_HERO_204")),
            ("BG25_008_e", Some("BG25_008")),
            ("BG_ShopBuff_Elemental_Ench", Some("BG_ShopBuff_Elemental")),
            ("TB_BaconShopBadsongE", Some("TB_BaconShopBadsong")),
            ("BG29_813e", Some("BG29_813")),
            // Nothing left, or not an id the report may carry.
            ("e", None),
            ("", None),
            ("BG 20e", None),
            ("BG20_GEM\u{e9}e", None),
        ];
        for (card, source) in cases {
            assert_eq!(source_of_enchantment(card).as_deref(), source, "{card}");
        }
    }

    #[test]
    fn an_id_without_a_suffix_is_its_own_source() {
        assert_eq!(
            source_of_enchantment("BG_Consumed").as_deref(),
            Some("BG_Consumed")
        );
    }

    #[test]
    fn buffs_are_the_visible_enchantments_in_play_on_the_minion_counted_by_card() {
        let b = board(vec![
            entity(10, "BG25_008", &[("CARDTYPE", "MINION")]),
            entity(11, "BG29_864t", &[("CARDTYPE", "MINION")]),
            enchantment(20, "BG20_GEMe2", 10, &[]),
            enchantment(21, "BG20_GEMe2", 10, &[]),
            enchantment(22, "BG31_816e", 10, &[]),
            enchantment(23, "BG20_GEMe2", 10, &[]),
            // The game's own holder of the Blood Gem numbers.
            enchantment(24, "BG20_GEMe", 10, &[("ENCHANTMENT_INVISIBLE", "1")]),
            // Dark Gifts and their stat holders have their own marks.
            enchantment(25, "BG36_MidGameEffect_000t9e2", 10, &[]),
            enchantment(26, "BG36_MidGameEffect_000t9e", 10, &[]),
            enchantment(27, "EDR_100t13e", 10, &[]),
            // A buff that left the minion, and one on another minion.
            {
                let mut gone = enchantment(28, "BG28_309e", 10, &[]);
                gone.tags
                    .insert("ZONE".into(), Value::Name("GRAVEYARD".into()));
                gone
            },
            enchantment(29, "BG28_503e", 11, &[]),
        ]);
        let ten = vec![buff("BG20_GEM", 3), buff("BG31_816", 1)];
        let index = buff_index(&b);
        assert_eq!(index.get(&10), Some(&ten));
        assert_eq!(index.get(&11), Some(&vec![buff("BG28_503", 1)]));
        assert_eq!(buffs_on(&b, 10), ten);
        assert!(buffs_on(&b, 12).is_empty());
    }

    #[test]
    fn two_enchantments_of_one_card_in_different_forms_are_one_kind() {
        let b = board(vec![
            entity(10, "BG25_008", &[("CARDTYPE", "MINION")]),
            enchantment(20, "BG34_950e", 10, &[]),
            enchantment(21, "BG34_950e2", 10, &[]),
        ]);
        assert_eq!(buffs_on(&b, 10), vec![buff("BG34_950", 2)]);
    }

    #[test]
    fn a_minion_keeps_at_most_twelve_kinds_and_999_of_one() {
        let mut entities = vec![entity(10, "BG25_008", &[("CARDTYPE", "MINION")])];
        for i in 0..15 {
            entities.push(enchantment(100 + i, &format!("BG25_{i:03}e"), 10, &[]));
        }
        assert_eq!(buffs_on(&board(entities), 10).len(), MAX_BUFF_KINDS);

        let mut many = vec![entity(10, "BG25_008", &[("CARDTYPE", "MINION")])];
        for i in 0..1005 {
            many.push(enchantment(100 + i, "BG20_GEMe2", 10, &[]));
        }
        assert_eq!(
            buffs_on(&board(many), 10),
            vec![buff("BG20_GEM", MAX_BUFF_COUNT)]
        );
    }
}
