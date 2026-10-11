use super::*;
use crate::cards::{rules, token, EFFECTS};

#[test]
fn every_gift_of_the_season_is_listed_once() {
    assert_eq!(GIFTS.len(), 47, "the card data has 47 gift spells");
    let mut seen = std::collections::HashSet::new();
    for g in GIFTS {
        assert!(g.id.starts_with("BG36_MidGameEffect_000t"), "{}", g.id);
        assert!(seen.insert(g.id), "{} is listed twice", g.id);
    }
    assert!(
        gift("BG36_MidGameEffect_010").is_none(),
        "the Discover is no gift"
    );
    assert!(
        gift("BG36_MidGameEffect_000t81e").is_none(),
        "enchantments are not keys"
    );
}

#[test]
fn every_token_a_gift_summons_is_known() {
    for g in GIFTS {
        for t in g.triggers {
            for a in t.actions() {
                if let Some(id) = a.token() {
                    assert!(token(id).is_some(), "{} summons unknown {id}", g.id);
                }
            }
        }
    }
}

#[test]
fn start_of_combat_gifts_are_known_as_such() {
    for id in [
        "000t7", "000t9", "000t16", "000t71", "000t81", "000t89", "000t90",
    ] {
        assert!(
            has_start_of_combat(&format!("BG36_MidGameEffect_{id}")),
            "{id}"
        );
    }
    assert!(!has_start_of_combat("BG36_MidGameEffect_000t61"));
    assert!(!has_start_of_combat("NOT_A_GIFT"));
}

#[test]
fn shop_only_gifts_change_nothing_in_combat() {
    let fortitude = gift("BG36_MidGameEffect_000t73").unwrap();
    assert!(fortitude.triggers.is_empty());
    assert_eq!(fortitude.keywords, Keywords::NONE);
    assert!(!fortitude.all_types && !fortitude.golden);
    assert_eq!(fortitude.fidelity, Fidelity::Exact);
}

#[test]
fn gilding_is_exact_only_when_the_golden_card_or_no_effect_is_known() {
    let gilding = gift("BG36_MidGameEffect_000t14").unwrap();
    // Bonehead's golden card is modelled; a keyword-only card has no effect.
    assert_eq!(fidelity(gilding, "BG28_300"), Fidelity::Exact);
    assert_eq!(fidelity(gilding, "BGS_119"), Fidelity::Exact);
    // Already golden: the card's own rules are the golden ones.
    assert_eq!(fidelity(gilding, "BG28_300_G"), Fidelity::Exact);
    let (no_golden, _) = EFFECTS
        .iter()
        .find(|(id, r)| {
            !id.ends_with("_G") && !r.triggers.is_empty() && rules(&golden_id(id)).is_none()
        })
        .expect("a modelled card whose golden card is not");
    assert!(matches!(
        fidelity(gilding, no_golden),
        Fidelity::Approximate(_)
    ));
}
