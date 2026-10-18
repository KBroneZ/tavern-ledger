//! Hero powers and trinkets in the engine's coverage (T-628, D-157): one
//! nobody reviewed blocks by name; one that acts at the start of combat
//! blocks only when start of combat is played; one that acts later in the
//! combat is approximate; one with no combat effect changes nothing. Also
//! the start-of-combat order between the two sides and Hopebringer's
//! growing buff.

use combat_sim::kit::{
    IN_COMBAT_NOT_PLAYED, NOT_REVIEWED, START_OF_COMBAT_NOT_PLAYED, TWICE_NOT_PLAYED,
};
use combat_sim::trials::SIDE_ORDER;
use combat_sim::{simulate, Kit, KitCard, Minion, Options, Verdict, Warband};

/// A keyword-only card the engine knows, so only the kit can make a gap.
fn plain(attack: i32, health: i32) -> Minion {
    Minion::new("BGS_119", attack, health).tier(1)
}

fn with_kit(minions: Vec<Minion>, powers: &[&str], trinkets: &[&str]) -> Warband {
    Warband {
        kit: Some(Kit {
            powers: powers.iter().map(|id| KitCard::new(*id)).collect(),
            trinkets: trinkets.iter().map(|id| KitCard::new(*id)).collect(),
        }),
        tavern_tier: Some(1),
        ..Warband::new(minions)
    }
}

fn options(start_of_combat: bool) -> Options {
    Options {
        trials: 200,
        start_of_combat,
        ..Options::default()
    }
}

fn verdict(player: &Warband, start_of_combat: bool) -> Verdict {
    let opponent = with_kit(vec![plain(1, 1)], &[], &[]);
    simulate(player, &opponent, &options(start_of_combat)).expect("valid boards")
}

#[test]
fn a_hero_power_nobody_reviewed_blocks_by_name() {
    let player = with_kit(vec![plain(2, 2)], &["BG99_HERO_999p"], &[]);
    for soc in [false, true] {
        let Verdict::NotModelled(cov) = verdict(&player, soc) else {
            panic!("an unknown hero power must block");
        };
        assert_eq!(cov.not_modelled, ["BG99_HERO_999p"]);
        assert_eq!(cov.kit, ["BG99_HERO_999p"]);
        assert_eq!(
            cov.reasons(),
            [("BG99_HERO_999p".to_string(), NOT_REVIEWED)]
        );
    }
}

#[test]
fn a_start_of_combat_trinket_blocks_only_when_start_of_combat_is_played() {
    // Holy Mallet: Start of Combat only.
    let player = with_kit(vec![plain(2, 2)], &[], &["BG30_MagicItem_902"]);
    let Verdict::Odds(odds) = verdict(&player, false) else {
        panic!("a board taken after start of combat already has it");
    };
    assert!(odds.coverage.is_exact(), "{:?}", odds.coverage);
    let Verdict::NotModelled(cov) = verdict(&player, true) else {
        panic!("its start of combat is not played yet");
    };
    assert_eq!(
        cov.reasons(),
        [("BG30_MagicItem_902".to_string(), START_OF_COMBAT_NOT_PLAYED)]
    );
}

#[test]
fn a_power_that_acts_later_in_the_combat_is_approximate() {
    // Glory of Combat and Deathly Phylactery act during the combat; Frostwolf
    // Fervor also when there is space later.
    let player = with_kit(
        vec![plain(2, 2)],
        &["BG20_HERO_100p"],
        &["BG30_MagicItem_700"],
    );
    let Verdict::Odds(odds) = verdict(&player, false) else {
        panic!("approximate, not blocked");
    };
    let gaps: Vec<_> = odds
        .coverage
        .approximate
        .iter()
        .map(|(id, why)| (id.as_str(), *why))
        .collect();
    assert_eq!(
        gaps,
        [
            ("BG20_HERO_100p", IN_COMBAT_NOT_PLAYED),
            ("BG30_MagicItem_700", IN_COMBAT_NOT_PLAYED)
        ]
    );
    let frostwolf = with_kit(vec![plain(2, 2)], &["BG22_HERO_002p"], &[]);
    let Verdict::Odds(odds) = verdict(&frostwolf, false) else {
        panic!("approximate on a board taken after start of combat");
    };
    assert_eq!(
        odds.coverage.approximate,
        [("BG22_HERO_002p".to_string(), IN_COMBAT_NOT_PLAYED)]
    );
    assert!(matches!(verdict(&frostwolf, true), Verdict::NotModelled(_)));
}

#[test]
fn ancient_wishbone_matters_only_with_a_combat_hero_power() {
    let shop_power = with_kit(
        vec![plain(2, 2)],
        &["TB_BaconShop_HP_046"],
        &["BG30_MagicItem_804"],
    );
    let Verdict::Odds(odds) = verdict(&shop_power, false) else {
        panic!()
    };
    assert!(odds.coverage.is_exact(), "{:?}", odds.coverage);
    let combat_power = with_kit(
        vec![plain(2, 2)],
        &["BG20_HERO_100p"],
        &["BG30_MagicItem_804"],
    );
    let Verdict::Odds(odds) = verdict(&combat_power, false) else {
        panic!()
    };
    assert!(odds
        .coverage
        .approximate
        .contains(&("BG30_MagicItem_804".to_string(), TWICE_NOT_PLAYED)));
}

#[test]
fn no_kit_or_a_kit_without_combat_effects_changes_nothing() {
    let bare = Warband {
        tavern_tier: Some(1),
        ..Warband::new(vec![plain(2, 2)])
    };
    let empty = with_kit(vec![plain(2, 2)], &[], &[]);
    // Gonna Be Rich! and both trinket placeholders: shop only.
    let shop = with_kit(
        vec![plain(2, 2)],
        &["TB_BaconShop_HP_046"],
        &["BG30_Trinket_1st", "BG30_Trinket_2nd"],
    );
    let odds = |w: &Warband| match verdict(w, true) {
        Verdict::Odds(o) => o,
        Verdict::NotModelled(c) => panic!("{c:?}"),
    };
    let base = odds(&bare);
    assert!(base.coverage.is_exact());
    for w in [&empty, &shop] {
        let o = odds(w);
        assert!(o.coverage.is_exact(), "{:?}", o.coverage);
        assert_eq!((o.won, o.tied, o.lost), (base.won, base.tied, base.lost));
    }
}

const HOSTILITY: &str = "BG36_MidGameEffect_000t71";

#[test]
fn start_of_combat_on_both_sides_names_the_order_as_unproven() {
    let player = with_kit(vec![plain(2, 2).gift(HOSTILITY)], &[], &[]);
    let opponent = with_kit(vec![plain(2, 3).gift(HOSTILITY)], &[], &[]);
    let Verdict::Odds(odds) = simulate(&player, &opponent, &options(true)).unwrap() else {
        panic!("both gifts are played");
    };
    assert_eq!(
        odds.coverage.approximate,
        [(HOSTILITY.to_string(), SIDE_ORDER)]
    );
    // One side only, or start of combat not played: nothing to order.
    let quiet = with_kit(vec![plain(2, 3)], &[], &[]);
    let Verdict::Odds(odds) = simulate(&player, &quiet, &options(true)).unwrap() else {
        panic!()
    };
    assert!(odds.coverage.is_exact(), "{:?}", odds.coverage);
    let Verdict::Odds(odds) = simulate(&player, &opponent, &options(false)).unwrap() else {
        panic!()
    };
    assert!(odds.coverage.is_exact(), "{:?}", odds.coverage);
}

#[test]
fn hopebringer_needs_a_board_taken_after_start_of_combat() {
    // Its buff grows during the game and the board does not give it.
    let player = with_kit(vec![Minion::new("BG36_364", 3, 3).tier(3)], &[], &[]);
    assert!(matches!(verdict(&player, true), Verdict::NotModelled(_)));
    let Verdict::Odds(odds) = verdict(&player, false) else {
        panic!("a board taken after it has the real buff");
    };
    assert!(!odds.coverage.approximate.is_empty());
}

#[test]
fn a_duos_teammate_brings_its_own_kit() {
    use combat_sim::{simulate_teams, Team};
    let first = with_kit(vec![plain(2, 2)], &["TB_BaconShop_HP_046"], &[]);
    let second = with_kit(vec![plain(2, 2)], &["BG99_HERO_999p"], &[]);
    let opponent = Team::solo(with_kit(vec![plain(1, 1)], &[], &[]));
    let verdict = simulate_teams(&Team::duo(first, second), &opponent, &options(false)).unwrap();
    let Verdict::NotModelled(cov) = verdict else {
        panic!("the teammate's unknown hero power blocks");
    };
    assert_eq!(cov.kit, ["BG99_HERO_999p"]);
}
