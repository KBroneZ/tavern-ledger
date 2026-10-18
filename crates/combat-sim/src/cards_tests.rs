use super::*;

fn all_effects() -> impl Iterator<Item = &'static (&'static str, CardRules)> {
    EFFECTS
        .iter()
        .chain(crate::cards_round2::EFFECTS_2)
        .chain(crate::cards_round3::EFFECTS_3)
        .chain(crate::cards_round4::EFFECTS_4)
        .chain(crate::cards_round5::EFFECTS_5)
        .chain(crate::cards_round7::EFFECTS_7)
        .chain(crate::cards_round8::EFFECTS_8)
}

fn no_combat_effect() -> impl Iterator<Item = &'static &'static str> {
    NO_COMBAT_EFFECT
        .iter()
        .chain(crate::cards_round8::NO_COMBAT_EFFECT_8)
}

#[test]
fn a_card_is_in_one_list_only() {
    for (id, _) in all_effects() {
        assert!(
            !no_combat_effect().any(|n| n == id),
            "{id} is in both lists"
        );
    }
    let mut seen = std::collections::HashSet::new();
    for id in no_combat_effect().chain(all_effects().map(|(id, _)| id)) {
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
                if matches!(action, Action::SummonRandomDeathrattle { .. }) {
                    for t in DEATHRATTLE_POOL {
                        assert!(token(t).is_some(), "{id} may summon unknown {t}");
                    }
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
    ("BGS_008", "TB_BaconUps_057"),
    ("BGS_123", "TB_BaconUps_162"),
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
    for id in no_combat_effect()
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
    // Tavern spells (build 254477) are cards of the hand: no golden card.
    const SPELLS: [&str; 4] = ["BG28_604", "BG28_698", "BG34_444", "BG34_689"];
    for id in no_combat_effect()
        .chain(all_effects().map(|(id, _)| id))
        .filter(|id| minion_card(id) && !id.ends_with("_G") && !SPELLS.contains(id))
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

/// The pool cards session 160 had not reviewed (T-616): each now plays or is
/// refused with a reason of its own, in both forms.
const REVIEWED_IN_ROUND_8: &[&str] = &[
    "BG23_000",
    "BG23_002",
    "BG23_004",
    "BG23_007",
    "BG23_008",
    "BG23_009",
    "BG25_013",
    "BG26_505",
    "BG27_514",
    "BG31_035",
    "BG31_812",
    "BG31_920",
    "BG31_924",
    "BG31_925",
    "BG31_999",
    "BG32_835",
    "BG32_837",
    "BG33_319",
    "BG34_322",
    "BG34_925",
    "BG35_341",
    "BG35_604",
    "BG35_921",
    "BG36_206",
    "BG36_308",
    "BG36_360",
    "BG36_508",
    "BG36_622",
    "BG36_921",
    "BGDUO31_209",
    "BGDUO_122",
    "BGDUO_125",
    "BGS_123",
    "BG_DEEP_015",
    "BG34_Giant_306",
    "BG36_360t4",
    "BGS_008",
];

#[test]
fn every_card_reviewed_in_round_8_is_listed_or_refused_in_both_forms() {
    for id in REVIEWED_IN_ROUND_8 {
        let golden = match *id {
            "BGS_008" => "TB_BaconUps_057".to_string(),
            "BGS_123" => "TB_BaconUps_162".to_string(),
            "BG36_360t4" => "BG36_360_Gt4".to_string(),
            _ => format!("{id}_G"),
        };
        for form in [id.to_string(), golden] {
            let refused = crate::cards_round8::REFUSED.iter().any(|(r, _)| *r == form);
            assert!(
                is_listed(&form) != refused,
                "{form} must be listed or refused, not both or neither"
            );
        }
    }
}

/// The 13 cards of the Battlegrounds patch of 2026-10-14 (build 254477) that
/// the engine did not list: what each is, and what the engine does with it.
#[test]
fn every_card_the_patch_changed_and_the_engine_did_not_list_is_sorted() {
    // Spells in the hand and trinkets with shop-only text: nothing to play
    // in combat. A Tavern spell has no golden card.
    for id in [
        "BG28_604",
        "BG28_698",
        "BG34_444",
        "BG34_689",
        "BG30_MagicItem_951",
        "BG32_MagicItem_232",
        "BG36_MagicItem_370",
        "BG36_MagicItem_430",
    ] {
        assert_eq!(rules(id), Some(PLAIN), "{id}");
        assert!(!is_listed(&format!("{id}_G")), "{id} has no golden card");
    }
    // Effects that act in combat but belong to no minion on a board:
    // refused, each with its own reason.
    for id in ["BG30_MagicItem_952", "BG30_MagicItem_989t", "BG_OldGod"] {
        assert_eq!(rules(id), None, "{id}");
        assert!(
            !refusal(id).contains("not reviewed"),
            "{id}: {}",
            refusal(id)
        );
    }
    assert!(refusal("BG30_MagicItem_952").contains("trinket"));
    assert!(refusal("BG30_MagicItem_989t").contains("trinket"));
    assert!(refusal("BG_OldGod").contains("Old God"));
    // Tentacle of C'Thun, both forms: a minion that gains stats in combat
    // when another friendly minion does; played as the plain minion, said.
    for id in ["TB_BaconShop_HERO_29_Buddy", "TB_BaconShop_HERO_29_Buddy_G"] {
        let r = rules(id).unwrap_or_else(|| panic!("{id} is not modelled"));
        assert!(r.triggers.is_empty(), "{id}");
        let Fidelity::Approximate(why) = r.fidelity else {
            panic!("{id} is exact");
        };
        assert!(why.contains("another friendly minion gains stats"), "{id}");
    }
}

#[test]
fn a_refused_card_says_why_and_a_card_nobody_looked_at_says_so() {
    assert!(refusal("BG34_Giant_590").contains("Undead Creations"));
    assert_eq!(refusal("BG34_Giant_590"), refusal("BG34_Giant_590_G"));
    assert!(refusal("NOT_A_CARD").contains("not reviewed"));
    assert_eq!(rules("BG34_Giant_590"), None);
}

#[test]
fn stitched_salvager_is_played_only_from_a_board_taken_before_start_of_combat() {
    let why = crate::cards::after_start_refusal("BG31_999").expect("a reason");
    assert!(why.contains("copies the minion it destroys"));
    assert_eq!(crate::cards::after_start_refusal("BG31_999_G"), Some(why));
    assert!(rules("BG31_999").is_some() && rules("BG31_999_G").is_some());
    assert_eq!(crate::cards::after_start_refusal("BG34_Giant_590"), None);
}

#[test]
fn ghastcoilers_pool_has_every_minion_once() {
    let mut seen = std::collections::HashSet::new();
    for id in DEATHRATTLE_POOL {
        assert!(seen.insert(*id), "{id} is in the pool twice");
        assert!(!id.ends_with("_G"), "{id} is golden");
    }
    assert!(!DEATHRATTLE_POOL.contains(&"BGS_008"));
    assert!(DEATHRATTLE_POOL.len() >= 45);
}

#[test]
fn only_a_build_the_rules_were_checked_against_passes_without_a_warning() {
    assert_eq!(unchecked_build(Some(254_477)), None);
    assert_eq!(unchecked_build(Some(253_216)), None);
    assert_eq!(unchecked_build(Some(999_999)), Some(999_999));
    // A build that was not read is not a build to warn about.
    assert_eq!(unchecked_build(None), None);
}

/// The numbers of the cards that act in combat, as the card texts of game
/// build 254477 give them (the patch of 2026-10-14 changed all of these).
#[test]
fn cards_that_act_in_combat_carry_the_numbers_of_build_254477() {
    // The actions of a card's first trigger.
    let first = |id: &str| -> Vec<Action> {
        let rules = rules(id).unwrap_or_else(|| panic!("{id} is not modelled"));
        rules.triggers[0].actions().to_vec()
    };
    let buff_all = |attack, health| Action::BuffAll {
        attack,
        health,
        tribe: Tribes::NONE,
        others: false,
    };
    // Ship Master Eudora: Deathrattle: give your minions +6/+6.
    assert_eq!(first("BG33_828"), [buff_all(6, 6)]);
    assert_eq!(first("BG33_828_G"), [buff_all(12, 12)]);
    // Hopebringer: Start of Combat: give your minions +4/+3 (printed; the
    // minion's own numbers when the board gives them, T-628).
    let hope = |attack, health| Action::BuffAllByCounters {
        attack,
        health,
        tribe: Tribes::NONE,
        others: false,
    };
    assert_eq!(first("BG36_364"), [hope(4, 3)]);
    assert_eq!(first("BG36_364_G"), [hope(8, 6)]);
    // Razorfen Vineweaver: Rally: 4 permanent Blood Gems on itself.
    for (id, count) in [("BG33_883", 4), ("BG33_883_G", 8)] {
        let gems = Action::Gems {
            to: GemTo::Itself,
            count,
        };
        assert_eq!(first(id), [gems], "{id}");
    }
    // Barrier Banshee: after a friendly Reborn, Divine Shield and +8/+8.
    for (id, n) in [("BG36_514", 8), ("BG36_514_G", 16)] {
        let gain = Action::GainStats {
            attack: n,
            health: n,
        };
        let shield = Action::GainKeyword(Keywords::DIVINE_SHIELD);
        assert_eq!(first(id), [shield, gain], "{id}");
    }
    // Ravaging Scorpid: after a friendly attack, Beetles have +4/+4 (golden
    // +8/+8).
    for (id, n) in [("BG36_209", 4), ("BG36_209_G", 8)] {
        let [Action::BuffCards { attack, health, .. }] = first(id)[..] else {
            panic!("{id} changed shape");
        };
        assert_eq!((attack, health), (n, n), "{id}");
    }
}

/// Cards whose text changed in build 254477 to something that happens in the
/// shop, the hand or at the end of the turn only: nothing to play in combat.
#[test]
fn cards_the_patch_moved_out_of_combat_have_no_combat_effect() {
    for id in [
        "BG34_682",   // Razorfen Flapper: Battlecry and Deathrattle: get a card
        "BG34_682_G", //
        "BG36_115",   // Nightmare Corroder: at the end of your turn, get a card
        "BG36_115_G", //
        "BG22_403",   // Young Murk-Eye: end of turn, Battlecries of neighbours
        "BG22_403_G", //
        "BG36_848",   // Lichling Hoarder: Avenge: get a plain copy
        "BG36_848_G", //
        "BG36_367",   // Auto Reveille: after you buy 3 cards
        "BG36_367_G", //
        "BG36_851",   // Spark Snapper: whenever you play a Mech
        "BG36_851_G", //
        "BGDUO_700",  // Voidpriest Cloner: after you discard a card
        "BGDUO_700_G",
    ] {
        assert_eq!(rules(id), Some(PLAIN), "{id}");
    }
    // Expert Aviator now summons a Murloc from the hand: the hand is not
    // seen, so the summon is still left out, and the card says so.
    for id in ["BG34_140", "BG34_140_G"] {
        assert!(
            matches!(
                rules(id).map(|r| r.fidelity),
                Some(Fidelity::Approximate(_))
            ),
            "{id}"
        );
    }
}
