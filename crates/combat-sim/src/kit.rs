//! What the engine knows of each hero power and trinket (T-628, D-157),
//! keyed by exact card id, like cards ([`crate::cards`]): a power or
//! trinket in none of the lists in [`crate::kit_lists`] blocks the odds by
//! name; one whose text never changes a combat is listed as such; one that
//! acts in combat is listed with when it acts.
//!
//! None of the effects is played yet: a board whose kit acts at the start of
//! combat must be taken after it (start of combat off, its effect already on
//! the board), and one that acts during combat makes the odds approximate.
//!
//! Written from the card texts in HearthstoneJSON (D-038) and the user's
//! logs; no simulator code was read (clean room).

use crate::kit_lists::{
    IN_COMBAT, NO_COMBAT_EFFECT, START_AND_IN_COMBAT, START_OF_COMBAT, TWICE_HERO_POWER,
};

/// When a hero power or trinket acts in a combat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KitEffect {
    /// Its text never changes a combat (the shop, the hand, the Tavern, a
    /// minion it gives, or a keyword the board already shows).
    None,
    /// At the start of combat only.
    StartOfCombat,
    /// At the start of combat, and perhaps again later in it.
    StartAndInCombat,
    /// During the combat, after its start.
    InCombat,
    /// Makes the hero's power trigger twice: it matters only with a power
    /// that acts in combat.
    TwiceHeroPower,
}

/// Why a hero power or trinket blocks the odds or makes them approximate.
pub const NOT_REVIEWED: &str = "the simulator has not reviewed this hero power or trinket yet";
pub const START_OF_COMBAT_NOT_PLAYED: &str = crate::trials::START_OF_COMBAT_GAP;
pub const IN_COMBAT_NOT_PLAYED: &str = "its effect during combat is not played yet";
pub const TWICE_NOT_PLAYED: &str = "it makes a hero power that acts in combat trigger twice";

/// What the engine knows of `card_id`; `None`: not reviewed.
pub fn effect(card_id: &str) -> Option<KitEffect> {
    let listed = |list: &[&str]| list.contains(&card_id);
    if listed(NO_COMBAT_EFFECT) {
        Some(KitEffect::None)
    } else if listed(START_OF_COMBAT) {
        Some(KitEffect::StartOfCombat)
    } else if listed(START_AND_IN_COMBAT) {
        Some(KitEffect::StartAndInCombat)
    } else if listed(IN_COMBAT) {
        Some(KitEffect::InCombat)
    } else if listed(TWICE_HERO_POWER) {
        Some(KitEffect::TwiceHeroPower)
    } else {
        None
    }
}

/// True when the hero power or trinket acts at the start of combat.
pub fn has_start_of_combat(card_id: &str) -> bool {
    matches!(
        effect(card_id),
        Some(KitEffect::StartOfCombat | KitEffect::StartAndInCombat)
    )
}

/// True when the card acts during the combat, after its start.
pub fn acts_after_start(card_id: &str) -> bool {
    matches!(
        effect(card_id),
        Some(KitEffect::InCombat | KitEffect::StartAndInCombat)
    )
}

/// True when the card acts in a combat at all (start of combat or during).
pub fn acts_in_combat(card_id: &str) -> bool {
    matches!(
        effect(card_id),
        Some(KitEffect::StartOfCombat | KitEffect::StartAndInCombat | KitEffect::InCombat)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_listed_id_is_in_one_list_only() {
        let lists: [&[&str]; 5] = [
            NO_COMBAT_EFFECT,
            START_OF_COMBAT,
            START_AND_IN_COMBAT,
            IN_COMBAT,
            TWICE_HERO_POWER,
        ];
        let mut seen = std::collections::BTreeSet::new();
        for list in lists {
            for id in list {
                assert!(seen.insert(*id), "{id} is listed twice");
            }
        }
    }

    #[test]
    fn a_power_or_trinket_nobody_reviewed_is_unknown() {
        assert_eq!(effect("BG99_HERO_999p"), None);
        assert_eq!(
            effect("TB_BaconShop_HP_037a"),
            Some(KitEffect::StartOfCombat)
        );
        assert_eq!(effect("BG30_MagicItem_700"), Some(KitEffect::InCombat));
        assert_eq!(effect("BG30_Trinket_1st"), Some(KitEffect::None));
        assert_eq!(
            effect("BG30_MagicItem_804"),
            Some(KitEffect::TwiceHeroPower)
        );
        assert!(has_start_of_combat("BG30_MagicItem_902"));
        assert!(!has_start_of_combat("BG30_MagicItem_700"));
        assert!(acts_in_combat("BG30_MagicItem_700"));
        // Frostwolf Fervor: at the start of combat and again when there is space.
        assert!(has_start_of_combat("BG22_HERO_002p") && acts_after_start("BG22_HERO_002p"));
        assert!(!acts_after_start("BG30_MagicItem_902"));
        assert!(!acts_in_combat("BG30_MagicItem_888"));
    }
}
