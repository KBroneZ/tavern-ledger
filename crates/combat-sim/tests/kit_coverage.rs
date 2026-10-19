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
            round: Some(7),
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
fn a_start_of_combat_power_not_played_yet_blocks_only_when_start_of_combat_is_played() {
    // Earth Invocation: Start of Combat only, not played yet.
    let player = with_kit(vec![plain(2, 2)], &["BG22_HERO_001p_t1"], &[]);
    let Verdict::Odds(odds) = verdict(&player, false) else {
        panic!("a board taken after start of combat already has it");
    };
    assert!(odds.coverage.is_exact(), "{:?}", odds.coverage);
    let Verdict::NotModelled(cov) = verdict(&player, true) else {
        panic!("its start of combat is not played yet");
    };
    assert_eq!(
        cov.reasons(),
        [("BG22_HERO_001p_t1".to_string(), START_OF_COMBAT_NOT_PLAYED)]
    );
}

#[test]
fn a_power_that_acts_later_in_the_combat_is_approximate() {
    // Broodmother and Blood Golem Sticker act during the combat (not played
    // yet); Frostwolf Fervor also when there is space later.
    let player = with_kit(
        vec![plain(2, 2)],
        &["BG22_HERO_305p"],
        &["BG30_MagicItem_442"],
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
            ("BG22_HERO_305p", IN_COMBAT_NOT_PLAYED),
            ("BG30_MagicItem_442", IN_COMBAT_NOT_PLAYED)
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
fn hopebringer_without_its_numbers_needs_a_board_taken_after_start_of_combat() {
    // Its buff grows during the game; without its numbers it is refused
    // before start of combat.
    let player = with_kit(vec![Minion::new("BG36_364", 3, 3).tier(3)], &[], &[]);
    assert!(matches!(verdict(&player, true), Verdict::NotModelled(_)));
    // A board taken after it already has the real buff: exact.
    let Verdict::Odds(odds) = verdict(&player, false) else {
        panic!("a board taken after start of combat");
    };
    assert!(odds.coverage.is_exact(), "{:?}", odds.coverage);
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

#[test]
fn a_played_start_of_combat_power_needs_no_board_after_it() {
    use combat_sim::kit::{COUNTER_NOT_GIVEN, LIGHTNING_REPEATS};
    // Wax Warband with its buff given (its third number): played, exact.
    let mut wax = KitCard::new("TB_BaconShop_HP_037a");
    wax.counters[2] = 3;
    let player = Warband {
        kit: Some(Kit {
            powers: vec![wax],
            trinkets: Vec::new(),
            round: Some(7),
        }),
        tavern_tier: Some(1),
        ..Warband::new(vec![plain(2, 2)])
    };
    let Verdict::Odds(odds) = verdict(&player, true) else {
        panic!("Wax Warband is played");
    };
    assert!(odds.coverage.is_exact(), "{:?}", odds.coverage);
    // Without its number the printed +1/+1 is used, named.
    let bare = with_kit(vec![plain(2, 2)], &["TB_BaconShop_HP_037a"], &[]);
    let Verdict::Odds(odds) = verdict(&bare, true) else {
        panic!()
    };
    assert_eq!(
        odds.coverage.approximate,
        [("TB_BaconShop_HP_037a".to_string(), COUNTER_NOT_GIVEN)]
    );
    // Lightning Invocation is played, approximate.
    let lightning = with_kit(vec![plain(2, 2)], &["BG22_HERO_001p_t4"], &[]);
    let Verdict::Odds(odds) = verdict(&lightning, true) else {
        panic!()
    };
    assert_eq!(
        odds.coverage.approximate,
        [("BG22_HERO_001p_t4".to_string(), LIGHTNING_REPEATS)]
    );
}

#[test]
fn hopebringer_is_played_before_start_of_combat_with_its_numbers() {
    let mut hope = Minion::new("BG36_364", 3, 3).tier(3);
    hope.counters = [12, 9];
    let player = with_kit(vec![hope], &[], &[]);
    let Verdict::Odds(odds) = verdict(&player, true) else {
        panic!("its own numbers are on the board");
    };
    assert!(odds.coverage.is_exact(), "{:?}", odds.coverage);
}

#[test]
fn with_start_of_combat_off_kits_and_numbers_change_no_odds() {
    // Boards taken after start of combat: every played power, trinket and
    // minion number is already on them.
    let mut hope = Minion::new("BG36_364", 3, 3).tier(3);
    hope.counters = [40, 30];
    let plain_hope = Minion::new("BG36_364", 3, 3).tier(3);
    let mut wax = KitCard::new("TB_BaconShop_HP_037a");
    wax.counters[2] = 9;
    let kitted = Warband {
        kit: Some(Kit {
            powers: vec![wax],
            trinkets: vec![KitCard::new("BG30_MagicItem_902")],
            round: Some(7),
        }),
        tavern_tier: Some(1),
        ..Warband::new(vec![hope, plain(2, 2)])
    };
    let bare = Warband {
        tavern_tier: Some(1),
        ..Warband::new(vec![plain_hope, plain(2, 2)])
    };
    let odds = |w: &Warband| match verdict(w, false) {
        Verdict::Odds(o) => (o.won, o.tied, o.lost, o.damage_dealt, o.damage_taken),
        Verdict::NotModelled(c) => panic!("{c:?}"),
    };
    assert_eq!(odds(&kitted), odds(&bare));
}

#[test]
fn runic_arcanists_spell_gap_matters_only_when_its_start_of_combat_is_played() {
    let arcanist = || Minion::new("BG36_245", 2, 4).tier(4);
    let player = with_kit(vec![arcanist()], &[], &[]);
    let Verdict::Odds(odds) = verdict(&player, false) else {
        panic!()
    };
    assert!(odds.coverage.is_exact(), "{:?}", odds.coverage);
    let Verdict::Odds(odds) = verdict(&player, true) else {
        panic!()
    };
    assert_eq!(odds.coverage.approximate.len(), 1, "{:?}", odds.coverage);
    // With the board's spell upgrades known it is exact again.
    let upgraded = Warband {
        values: combat_sim::GameValues {
            spell: (1, 2),
            ..Default::default()
        },
        ..with_kit(vec![arcanist()], &[], &[])
    };
    let Verdict::Odds(odds) = verdict(&upgraded, true) else {
        panic!()
    };
    assert!(odds.coverage.is_exact(), "{:?}", odds.coverage);
}

/// A board of plain minions with one hero power, in `round`.
fn with_power(minions: Vec<Minion>, card: KitCard, round: Option<u32>) -> Warband {
    Warband {
        kit: Some(Kit {
            powers: vec![card],
            trinkets: Vec::new(),
            round,
        }),
        tavern_tier: Some(1),
        ..Warband::new(minions)
    }
}

fn ready(id: &str) -> KitCard {
    let mut card = KitCard::new(id);
    card.counters[0] = 1;
    card
}

fn blocked_for(player: &Warband) -> Vec<(String, &'static str)> {
    match verdict(player, true) {
        Verdict::NotModelled(cov) => cov.reasons(),
        Verdict::Odds(o) => panic!("not blocked: {:?}", o.coverage),
    }
}

fn gaps(player: &Warband, start_of_combat: bool) -> Vec<(String, &'static str)> {
    match verdict(player, start_of_combat) {
        Verdict::Odds(o) => o.coverage.approximate,
        Verdict::NotModelled(cov) => panic!("blocked: {:?}", cov.reasons()),
    }
}

#[test]
fn frostwolf_fervor_is_played_from_round_seven_and_needs_the_round() {
    use combat_sim::kit::{COPY_OF_DAMAGED, ROUND_NOT_GIVEN};
    let at = |round| with_power(vec![plain(2, 2)], ready("BG22_HERO_002p"), round);
    assert_eq!(
        gaps(&at(Some(7)), true),
        [("BG22_HERO_002p".to_string(), COPY_OF_DAMAGED)]
    );
    // Locked before round 7: nothing to play, nothing to name.
    assert!(gaps(&at(Some(6)), true).is_empty());
    assert_eq!(
        blocked_for(&at(None)),
        [("BG22_HERO_002p".to_string(), ROUND_NOT_GIVEN)]
    );
    // A board taken after start of combat: its later copies are not played.
    assert_eq!(
        gaps(&at(None), false),
        [("BG22_HERO_002p".to_string(), IN_COMBAT_NOT_PLAYED)]
    );
}

#[test]
fn targeted_and_holding_powers_need_what_they_name() {
    use combat_sim::kit::{HELD_NOT_GIVEN, TARGET_NOT_GIVEN, TEAMMATE_NOT_GIVEN};
    let rr = with_power(vec![plain(2, 2)], KitCard::new("BG25_HERO_103p"), Some(7));
    assert_eq!(
        blocked_for(&rr),
        [("BG25_HERO_103p".to_string(), TARGET_NOT_GIVEN)]
    );
    let mut named = KitCard::new("BG25_HERO_103p");
    named.target = Some(0);
    assert!(gaps(&with_power(vec![plain(2, 2)], named, Some(7)), true).is_empty());
    let lnl = with_power(vec![plain(2, 2)], ready("BG22_HERO_000p_Alt"), Some(7));
    assert_eq!(
        blocked_for(&lnl),
        [("BG22_HERO_000p_Alt".to_string(), HELD_NOT_GIVEN)]
    );
    // Holding nothing (its first number at 0): nothing to fire.
    let empty = with_power(
        vec![plain(2, 2)],
        KitCard::new("BG22_HERO_000p_Alt"),
        Some(7),
    );
    assert!(gaps(&empty, true).is_empty());
    // Glorious Gloop in Solo: no teammate to copy from.
    let mut gloop = KitCard::new("BGDUO_HERO_101p");
    gloop.target = Some(0);
    let gloop = with_power(vec![plain(2, 2)], gloop, Some(7));
    assert_eq!(
        blocked_for(&gloop),
        [("BGDUO_HERO_101p".to_string(), TEAMMATE_NOT_GIVEN)]
    );
}

#[test]
fn sandy_needs_its_teammates_board_and_is_approximate_with_it() {
    use combat_sim::cards::SANDY_COPY;
    use combat_sim::kit::TEAMMATE_NOT_GIVEN;
    use combat_sim::{simulate_teams, Team};
    let sandy = with_kit(vec![Minion::new("BGDUO_125", 1, 1).tier(7)], &[], &[]);
    assert_eq!(
        blocked_for(&sandy),
        [("BGDUO_125".to_string(), TEAMMATE_NOT_GIVEN)]
    );
    let mate = with_kit(vec![plain(3, 9)], &[], &[]);
    let opponent = Team::solo(with_kit(vec![plain(1, 1)], &[], &[]));
    let team = Team::duo(sandy.clone(), mate);
    let Ok(Verdict::Odds(odds)) = simulate_teams(&team, &opponent, &options(true)) else {
        panic!("played with the teammate's board");
    };
    assert_eq!(
        odds.coverage.approximate,
        [("BGDUO_125".to_string(), SANDY_COPY)]
    );
    // A board taken after start of combat: nothing of it is played.
    let Verdict::Odds(odds) = verdict(&sandy, false) else {
        panic!()
    };
    assert!(odds.coverage.is_exact(), "{:?}", odds.coverage);
}

#[test]
fn stitched_salvager_and_timewarped_chameleon_are_played_before_start_of_combat() {
    let salvager = with_kit(
        vec![plain(2, 2), Minion::new("BG31_999", 16, 4).tier(7)],
        &[],
        &[],
    );
    let Verdict::Odds(odds) = verdict(&salvager, true) else {
        panic!("played from a board taken before start of combat");
    };
    assert!(odds.coverage.is_exact(), "{:?}", odds.coverage);
    let Verdict::NotModelled(cov) = verdict(&salvager, false) else {
        panic!("a board taken after it no longer has the destroyed minion");
    };
    assert!(cov.reasons()[0].1.contains("copies the minion it destroys"));
    let chameleon = with_kit(
        vec![plain(2, 2), Minion::new("BG34_Giant_042", 6, 15).tier(5)],
        &[],
        &[],
    );
    for soc in [false, true] {
        let Verdict::Odds(odds) = verdict(&chameleon, soc) else {
            panic!("Timewarped Chameleon is known");
        };
        assert!(odds.coverage.is_exact(), "{:?}", odds.coverage);
    }
}

#[test]
fn the_minion_lock_and_load_holds_is_checked_like_a_board_minion() {
    use combat_sim::kit::FIRED_AT_RIGHT_END;
    let mut lnl = ready("BG22_HERO_000p_Alt");
    lnl.held = Some(plain(4, 3));
    let known = with_power(vec![plain(2, 2)], lnl.clone(), Some(7));
    assert_eq!(
        gaps(&known, true),
        [("BG22_HERO_000p_Alt".to_string(), FIRED_AT_RIGHT_END)]
    );
    lnl.held = Some(Minion::new("ZZ_NOBODY_KNOWS", 4, 3));
    let unknown = with_power(vec![plain(2, 2)], lnl, Some(7));
    let Verdict::NotModelled(cov) = verdict(&unknown, true) else {
        panic!("a held card nobody reviewed blocks");
    };
    assert_eq!(cov.not_modelled, ["ZZ_NOBODY_KNOWS"]);
    // On a board taken after start of combat it is not fired: not checked.
    assert!(matches!(verdict(&unknown, false), Verdict::Odds(_)));
}

#[test]
fn whirl_o_tron_stays_approximate_on_a_board_taken_after_start_of_combat() {
    let whirl = with_kit(vec![Minion::new("BG34_Giant_599", 7, 5).tier(5)], &[], &[]);
    let Verdict::Odds(odds) = verdict(&whirl, false) else {
        panic!()
    };
    assert!(
        odds.coverage
            .approximate
            .iter()
            .any(|(id, _)| id == "BG34_Giant_599"),
        "{:?}",
        odds.coverage
    );
}

#[test]
fn a_held_minions_dark_gift_is_checked_too() {
    let mut lnl = ready("BG22_HERO_000p_Alt");
    lnl.held = Some(plain(4, 3).gift("BG36_MidGameEffect_ZZ_UNKNOWN"));
    let player = with_power(vec![plain(2, 2)], lnl, Some(7));
    let Verdict::NotModelled(cov) = verdict(&player, true) else {
        panic!("an unknown gift blocks");
    };
    assert_eq!(cov.not_modelled, ["BG36_MidGameEffect_ZZ_UNKNOWN"]);
    // Another power's minion named off the board is no held minion.
    let mut rr = KitCard::new("BG25_HERO_103p");
    rr.target = Some(0);
    rr.held = Some(Minion::new("ZZ_NOBODY_KNOWS", 1, 1));
    let player = with_power(vec![plain(2, 2)], rr, Some(7));
    assert!(matches!(verdict(&player, true), Verdict::Odds(_)));
}

#[test]
fn powers_and_trinkets_played_during_combat_name_only_their_own_gaps() {
    use combat_sim::kit::{SCALE_ON_SHIELDED, SPROUT_REBORN};
    // Glory of Combat, Jar o' Gems (with the side's Blood Gem upgrades) and
    // Lionfish Portrait: played, exact, on either board.
    let mut exact = with_kit(
        vec![plain(2, 2)],
        &["BG20_HERO_100p"],
        &["BG30_MagicItem_546", "BG36_MagicItem_201"],
    );
    exact.values.blood_gem = (1, 1);
    for soc in [false, true] {
        assert!(gaps(&exact, soc).is_empty(), "start of combat {soc}");
    }
    // Without the upgrades, Jar o' Gems is approximate like a Blood Gem card.
    exact.values.blood_gem = (0, 0);
    let jar = gaps(&exact, true);
    assert_eq!(jar.len(), 1);
    assert_eq!(jar[0].0, "BG30_MagicItem_546");
    assert!(jar[0].1.contains("Blood Gems"));
    let rough = with_kit(
        vec![plain(2, 2)],
        &["TB_BaconShop_HP_107"],
        &["BG32_MagicItem_363"],
    );
    assert_eq!(
        gaps(&rough, false),
        [
            ("TB_BaconShop_HP_107".to_string(), SPROUT_REBORN),
            ("BG32_MagicItem_363".to_string(), SCALE_ON_SHIELDED)
        ]
    );
    // Deathly Phylactery: a start of combat may have used it before a board
    // taken at the first attack, so only played before start of combat.
    let phylactery = with_kit(vec![plain(2, 2)], &[], &["BG30_MagicItem_700"]);
    assert!(gaps(&phylactery, true).is_empty());
    assert_eq!(
        gaps(&phylactery, false),
        [("BG30_MagicItem_700".to_string(), IN_COMBAT_NOT_PLAYED)]
    );
}

#[test]
fn the_powers_and_trinkets_reviewed_in_t631_with_no_combat_effect_change_nothing() {
    // A Tale of Kings' forms; Comfy Coffin, Surprise Portrait, Bleeding Heart
    // (its Undead goes to the hand), Upstart Embers (the shop), Gold Mallet
    // (end of turn).
    let powers = [
        "TB_BaconShop_HP_041a",
        "TB_BaconShop_HP_041f",
        "TB_BaconShop_HP_041i",
        "TB_BaconShop_HP_041l",
    ];
    let trinkets = [
        "BG30_MagicItem_547t",
        "BG30_MagicItem_555",
        "BG30_MagicItem_713",
        "BG35_MagicItem_862",
        "BG36_MagicItem_302",
    ];
    for power in powers {
        for soc in [false, true] {
            let player = with_kit(vec![plain(2, 2)], &[power], &trinkets);
            let Verdict::Odds(odds) = verdict(&player, soc) else {
                panic!("{power}: reviewed, no combat effect");
            };
            assert!(odds.coverage.is_exact(), "{:?}", odds.coverage);
        }
    }
}

#[test]
fn fragrant_phylactery_is_played_before_start_of_combat_and_read_from_its_mark_after() {
    use combat_sim::gifts::PHYLACTERY_MARK;
    use combat_sim::kit::{LOWEST_TIES, MARK_NOT_GIVEN};
    let power = "BG20_HERO_282p";
    // Before start of combat: played, approximate on equal Attack.
    let before = with_kit(vec![plain(2, 2), plain(3, 3)], &[power], &[]);
    let Verdict::Odds(odds) = verdict(&before, true) else {
        panic!("played from before start of combat");
    };
    assert_eq!(
        odds.coverage.approximate,
        [(power.to_string(), LOWEST_TIES)]
    );
    // After it, without the mark on any minion: the minion it chose is not
    // known, so the odds are blocked by name.
    let Verdict::NotModelled(cov) = verdict(&before, false) else {
        panic!("its mark is not shown");
    };
    assert_eq!(cov.reasons(), [(power.to_string(), MARK_NOT_GIVEN)]);
    // With the mark shown: exact.
    let after = with_kit(
        vec![plain(2, 2).gift(PHYLACTERY_MARK), plain(3, 3)],
        &[power],
        &[],
    );
    let Verdict::Odds(odds) = verdict(&after, false) else {
        panic!("the mark is shown");
    };
    assert!(odds.coverage.is_exact(), "{:?}", odds.coverage);
}

#[test]
fn eternal_portrait_is_played_and_approximate_and_another_block_stays() {
    use combat_sim::kit::KNIGHTS_SUMMONED;
    let portrait = with_kit(vec![plain(2, 2)], &[], &["BG36_MagicItem_216"]);
    let Verdict::Odds(odds) = verdict(&portrait, false) else {
        panic!("played");
    };
    assert_eq!(
        odds.coverage.approximate,
        [("BG36_MagicItem_216".to_string(), KNIGHTS_SUMMONED)]
    );
    // Playing one card never lifts another one's block.
    let both = with_kit(
        vec![plain(2, 2)],
        &["BG99_HERO_999p", "BG20_HERO_282p"],
        &["BG36_MagicItem_216"],
    );
    for soc in [false, true] {
        let Verdict::NotModelled(cov) = verdict(&both, soc) else {
            panic!("the unknown power still blocks");
        };
        assert!(cov.not_modelled.contains(&"BG99_HERO_999p".to_string()));
    }
}
