use super::*;

fn all_effects() -> impl Iterator<Item = &'static (&'static str, CardRules)> {
    EFFECTS
        .iter()
        .chain(crate::cards_round2::EFFECTS_2)
        .chain(crate::cards_round3::EFFECTS_3)
        .chain(crate::cards_round4::EFFECTS_4)
        .chain(crate::cards_round5::EFFECTS_5)
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

/// Cards the game lists in a golden form with another id pattern: normal id,
/// golden id.
const OLD_STYLE_TWINS: &[(&str, &str)] = &[
    ("BGS_018", "TB_BaconUps_085"),
    ("BGS_034", "TB_BaconUps_149"),
    ("BGS_040", "TB_BaconUps_154"),
    ("BGS_115", "TB_BaconUps_156"),
    ("BGS_116", "TB_BaconUps_167"),
    ("BGS_131", "TB_BaconUps_251"),
];

fn is_listed(id: &str) -> bool {
    rules(id).is_some()
}

#[test]
fn the_two_cards_of_the_report_are_known_in_both_forms() {
    // Disguised Graverobber and Spark Snapper: one form of each was listed.
    for id in ["BG28_303", "BG28_303_G", "BG36_851", "BG36_851_G"] {
        assert!(is_listed(id), "{id} is not modelled");
    }
}

#[test]
fn a_golden_card_is_never_listed_without_its_normal_form() {
    for id in NO_COMBAT_EFFECT
        .iter()
        .chain(all_effects().map(|(id, _)| id))
        .filter_map(|id| id.strip_suffix("_G"))
    {
        assert!(is_listed(id), "{id}_G is listed but {id} is not");
    }
}

/// Minion cards named like `BG36_851` or `BGDUO_107` all have a golden form,
/// `BG36_851_G`; a card listed in one form only is a combat the simulator
/// refuses for a reason that does not show on the board.
#[test]
fn a_normal_card_is_never_listed_without_its_golden_form() {
    let minion_card = |id: &str| {
        let (set, number) = id.split_once('_').unwrap_or_default();
        set.starts_with("BG")
            && !set.starts_with("BGS")
            && set[2..]
                .chars()
                .all(|c| c.is_ascii_digit() || c == 'D' || c == 'U' || c == 'O')
            && !number.is_empty()
            && number.chars().all(|c| c.is_ascii_digit())
    };
    for id in NO_COMBAT_EFFECT
        .iter()
        .chain(all_effects().map(|(id, _)| id))
        .filter(|id| minion_card(id) && !id.ends_with("_G"))
    {
        assert!(
            is_listed(&format!("{id}_G")),
            "{id} is listed but {id}_G is not"
        );
    }
}

#[test]
fn old_style_golden_cards_are_listed_with_their_normal_form() {
    for (normal, golden) in OLD_STYLE_TWINS {
        assert!(is_listed(normal), "{normal} is not modelled");
        assert!(is_listed(golden), "{golden} is not modelled");
    }
}
