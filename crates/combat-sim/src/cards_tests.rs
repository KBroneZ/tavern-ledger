use super::*;

fn all_effects() -> impl Iterator<Item = &'static (&'static str, CardRules)> {
    EFFECTS
        .iter()
        .chain(crate::cards_round2::EFFECTS_2)
        .chain(crate::cards_round3::EFFECTS_3)
        .chain(crate::cards_round4::EFFECTS_4)
}

#[test]
fn a_card_is_in_one_list_only() {
    for (id, _) in all_effects() {
        assert!(!NO_COMBAT_EFFECT.contains(id), "{id} is in both lists");
    }
    let mut seen = std::collections::HashSet::new();
    for id in NO_COMBAT_EFFECT
        .iter()
        .chain(all_effects().map(|(id, _)| id))
    {
        assert!(seen.insert(*id), "{id} is listed twice");
    }
}

#[test]
fn every_summoned_token_is_known() {
    for (id, rules) in all_effects() {
        for trigger in rules.triggers {
            for action in trigger.actions() {
                if let Some(t) = action.token() {
                    assert!(token(t).is_some(), "{id} summons unknown {t}");
                }
            }
        }
    }
}

#[test]
fn start_of_combat_cards_are_known_as_such() {
    let soc = EFFECTS
        .iter()
        .find(|(_, r)| r.triggers.contains(&Trigger::StartOfCombatUnmodelled))
        .map(|(id, _)| *id)
        .expect("some card acts at the start of combat");
    assert!(has_start_of_combat(soc));
    assert!(!has_start_of_combat("BGS_119"));
    assert!(!has_start_of_combat("NOT_A_CARD"));
}

#[test]
fn unknown_cards_are_not_modelled() {
    assert_eq!(rules("NOT_A_CARD"), None);
    assert_eq!(rules("BGS_119"), Some(PLAIN));
    assert!(matches!(
        rules("BG36_202").map(|r| r.fidelity),
        Some(Fidelity::Approximate(_))
    ));
}
