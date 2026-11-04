//! Card round 8 (T-616): the cards that act in combat, from their card
//! texts, on small boards whose outcome is certain.

use super::{Combat, Prepared};
use crate::model::{Keywords, Minion, Tribes, Warband};
use crate::rng::Rng;

fn band(minions: Vec<Minion>) -> Warband {
    Warband {
        kit: None,
        minions,
        tavern_tier: Some(1),
        values: Default::default(),
    }
}

/// A keyword-only card, so the boards below are fully modelled.
fn plain(attack: i32, health: i32) -> Minion {
    Minion::new("BGS_119", attack, health).tier(1)
}

fn with<R>(player: Vec<Minion>, opponent: Vec<Minion>, f: impl FnOnce(&mut Combat) -> R) -> R {
    let p = Prepared::new(&band(player), &band(opponent), false);
    let mut rng = Rng::new(7);
    let mut c = Combat::new(&p, &mut rng);
    f(&mut c)
}

fn stats(c: &Combat, side: usize) -> Vec<(i32, i32)> {
    c.sides[side]
        .units
        .iter()
        .map(|u| (u.attack, u.health))
        .collect()
}

fn ids(c: &Combat, side: usize) -> Vec<String> {
    c.sides[side]
        .units
        .iter()
        .map(|u| c.p.cards[u.card as usize].id.clone())
        .collect()
}

fn uid_at(c: &Combat, side: usize, ix: usize) -> u32 {
    c.sides[side].units[ix].uid
}

/// Kills the minions of side 0 at `ixs` at once (a hit from the first
/// enemy) and resolves the deaths.
fn kill(c: &mut Combat, ixs: &[usize]) {
    let by = uid_at(c, 1, 0);
    for &ix in ixs {
        let (uid, health) = (uid_at(c, 0, ix), c.sides[0].units[ix].health);
        c.damage(0, uid, health, Keywords::NONE, by);
    }
    c.process_deaths(1);
}

fn target() -> Vec<Minion> {
    vec![plain(0, 99)]
}

// Timewarped Jazzer: Deathrattle: your Blood Gems give an extra +2 Health
// this game (golden +4).

#[test]
fn timewarped_jazzer_upgrades_the_blood_gems_played_after_it() {
    for (jazzer, extra) in [("BG34_Giant_306", 2), ("BG34_Giant_306_G", 4)] {
        let jazzer = Minion::new(jazzer, 5, 3).tier(3);
        // Tusked Camper: Rally, a Blood Gem on itself.
        let camper = Minion::new("BG33_886", 1, 10).tier(1);
        with(vec![jazzer, camper], target(), |c| {
            kill(c, &[0]);
            assert_eq!(c.sides[0].values.blood_gem, (0, extra));
            let uid = uid_at(c, 0, 0);
            c.swing(0, uid);
            assert_eq!(stats(c, 0), vec![(2, 10 + 1 + extra)]);
        });
    }
}

// Dark Paradox (a tier 5 form): its Divine Shield takes 3 hits to break
// (golden 4).

#[test]
fn dark_paradox_shield_takes_three_hits_and_golden_four() {
    for (id, hits) in [("BG36_360t4", 3), ("BG36_360_Gt4", 4)] {
        let paradox = Minion::new(id, 8, 4)
            .with(Keywords::DIVINE_SHIELD)
            .tribe(Tribes::ALL)
            .tier(5);
        with(vec![paradox], target(), |c| {
            let (uid, by) = (uid_at(c, 0, 0), uid_at(c, 1, 0));
            for _ in 1..hits {
                c.damage(0, uid, 5, Keywords::NONE, by);
                assert!(c.sides[0].units[0].kw.contains(Keywords::DIVINE_SHIELD));
            }
            c.damage(0, uid, 5, Keywords::NONE, by);
            assert!(!c.sides[0].units[0].kw.contains(Keywords::DIVINE_SHIELD));
            assert_eq!(stats(c, 0), vec![(8, 4)]);
        });
    }
}

// Rot Hide Gnoll: has +1 Attack for each friendly minion that died this
// combat (golden +2).

fn gnoll(id: &str, attack: i32) -> Minion {
    Minion::new(id, attack, 4).tribe(Tribes::UNDEAD).tier(1)
}

#[test]
fn rot_hide_gnoll_gains_attack_for_each_friendly_death() {
    for (id, base, per) in [("BG25_013", 1, 1), ("BG25_013_G", 2, 2)] {
        with(
            vec![gnoll(id, base), plain(1, 1), plain(1, 1), plain(1, 5)],
            target(),
            |c| {
                kill(c, &[1, 2]);
                assert_eq!(stats(c, 0), vec![(base + 2 * per, 4), (1, 5)]);
                kill(c, &[1]);
                assert_eq!(stats(c, 0), vec![(base + 3 * per, 4)]);
            },
        );
    }
}

#[test]
fn a_gnoll_that_comes_back_counts_the_deaths_so_far() {
    // The Gnoll has Reborn: it dies with one friend (two deaths), and its
    // fresh copy has the card's 1 Attack and 1 Health plus +1 for each.
    let mut g = gnoll("BG25_013", 3).with(Keywords::REBORN);
    g.base = Some(crate::model::BaseCard {
        attack: 1,
        health: 4,
        keywords: Keywords::NONE,
    });
    with(vec![g, plain(1, 1)], target(), |c| {
        kill(c, &[0, 1]);
        assert_eq!(stats(c, 0), vec![(3, 1)]);
    });
}

// Sewer Lord: Deathrattle: summon two Sewer Rats that summon 2/3 Turtles
// with Taunt (golden: Golden Sewer Rats, 4/6 Turtles).

#[test]
fn sewer_lord_summons_two_rats_that_summon_turtles() {
    for (lord, rat, rat_stats, turtle, turtle_stats) in [
        ("BG35_604", "BG19_010", (3, 2), "BG19_010t", (2, 3)),
        ("BG35_604_G", "BG19_010_G", (6, 4), "BG19_010_Gt", (4, 6)),
    ] {
        let lord = Minion::new(lord, 4, 6).tribe(Tribes::BEAST).tier(5);
        with(vec![lord], target(), |c| {
            kill(c, &[0]);
            assert_eq!(ids(c, 0), vec![rat, rat]);
            assert_eq!(stats(c, 0), vec![rat_stats, rat_stats]);
            kill(c, &[0]);
            assert_eq!(ids(c, 0), vec![turtle, rat]);
            assert_eq!(stats(c, 0)[0], turtle_stats);
            assert!(c.sides[0].units[0].kw.contains(Keywords::TAUNT));
        });
    }
}

// Stalwart Kodo: after you summon a minion in combat, give it this minion's
// maximum stats, 3 times per combat (golden: double).

fn kodo(id: &str, attack: i32, health: i32) -> Minion {
    Minion::new(id, attack, health).tribe(Tribes::BEAST).tier(7)
}

fn bonehead(id: &str) -> Minion {
    Minion::new(id, 1, 1).tribe(Tribes::UNDEAD).tier(1)
}

#[test]
fn stalwart_kodo_gives_summoned_minions_double_its_maximum_stats() {
    // Bonehead's Deathrattle summons two 1/1 Skeletons: each gets double the
    // Kodo's Attack and its full Health (not what the Kodo has left).
    with(
        vec![kodo("BG34_322", 15, 30), bonehead("BG28_300")],
        target(),
        |c| {
            let (kodo, by) = (uid_at(c, 0, 0), uid_at(c, 1, 0));
            c.damage(0, kodo, 20, Keywords::NONE, by);
            kill(c, &[1]);
            assert_eq!(stats(c, 0), vec![(15, 10), (31, 61), (31, 61)]);
        },
    );
}

#[test]
fn golden_kodo_gives_triple_and_only_three_times_per_combat() {
    // Golden Bonehead summons four Skeletons: three get triple stats.
    with(
        vec![kodo("BG34_322_G", 30, 60), bonehead("BG28_300_G")],
        target(),
        |c| {
            kill(c, &[1]);
            assert_eq!(
                stats(c, 0),
                vec![(30, 60), (91, 181), (91, 181), (91, 181), (1, 1)]
            );
        },
    );
}

// Showy Cyclist: Deathrattle: give all your Naga +2/+1 (golden +4/+2).

#[test]
fn showy_cyclist_buffs_the_nagas_when_it_dies() {
    for (id, (attack, health)) in [("BG31_925", (2, 1)), ("BG31_925_G", (4, 2))] {
        let cyclist = Minion::new(id, 4, 2).tribe(Tribes::NAGA).tier(5);
        let naga = plain(3, 3).tribe(Tribes::NAGA);
        // Minions share a card's facts by its id, so the other tribe needs
        // another card.
        let other = Minion::new("BGS_034", 3, 3).tier(1);
        with(vec![cyclist, naga, other], target(), |c| {
            kill(c, &[0]);
            assert_eq!(stats(c, 0), vec![(3 + attack, 3 + health), (3, 3)]);
        });
    }
}

// Ghastcoiler: Deathrattle: summon 2 random Deathrattle minions (golden 4).

#[test]
fn ghastcoiler_summons_random_deathrattle_minions() {
    for (id, count) in [("BGS_008", 2), ("TB_BaconUps_057", 4)] {
        let coiler = Minion::new(id, 7, 7).tribe(Tribes::BEAST).tier(5);
        with(vec![coiler], target(), |c| {
            kill(c, &[0]);
            let summoned = ids(c, 0);
            assert_eq!(summoned.len(), count);
            for card in &summoned {
                assert!(crate::cards::DEATHRATTLE_POOL.contains(&card.as_str()));
            }
        });
    }
}

#[test]
fn ghastcoiler_summons_only_what_fits_on_the_board() {
    let coiler = Minion::new("TB_BaconUps_057", 7, 7)
        .tribe(Tribes::BEAST)
        .tier(5);
    let mut board = vec![coiler];
    board.extend((0..5).map(|_| plain(1, 9)));
    with(board, target(), |c| {
        kill(c, &[0]);
        assert_eq!(c.sides[0].units.len(), 7);
    });
}

/// Tentacle of C'Thun (build 254477): the odds come back, with the card named
/// as approximate and its reason, in both forms.
#[test]
fn tentacle_of_cthun_plays_as_its_printed_minion_and_is_named_approximate() {
    use crate::trials::{simulate, Options, Verdict};
    for (id, attack) in [
        ("TB_BaconShop_HERO_29_Buddy", 3),
        ("TB_BaconShop_HERO_29_Buddy_G", 6),
    ] {
        let tentacle = Minion::new(id, attack, attack).tier(2);
        let foe = plain(1, 1);
        let Verdict::Odds(o) =
            simulate(&band(vec![tentacle]), &band(vec![foe]), &Options::default())
                .expect("a valid board")
        else {
            panic!("{id} is refused");
        };
        assert_eq!(o.won, o.trials, "{id}");
        assert!(
            o.coverage
                .approximate
                .iter()
                .any(|(c, why)| c == id && why.contains("another friendly minion gains stats")),
            "{id}"
        );
    }
}

// Engine 0.23.0: five cards whose odds were refused.

fn printed(mut m: Minion, attack: i32, health: i32) -> Minion {
    m.base = Some(crate::model::BaseCard {
        attack,
        health,
        keywords: Keywords::NONE,
    });
    m
}

fn lost_spirit(id: &str, attack: i32, health: i32) -> Minion {
    let spirit = Minion::new(id, attack, health)
        .tribe(Tribes::UNDEAD)
        .tier(1);
    printed(spirit, 1, 1)
}

/// What the odds say is approximate, start of combat off.
fn coverage(player: Vec<Minion>) -> crate::trials::Coverage {
    use crate::trials::{simulate, Options, Verdict};
    let options = Options {
        trials: 50,
        ..Options::default()
    };
    match simulate(&band(player), &band(target()), &options).expect("a valid board") {
        Verdict::Odds(o) => o.coverage,
        Verdict::NotModelled(c) => panic!("refused: {:?}", c.not_modelled),
    }
}

fn reason(c: &crate::trials::Coverage, id: &str) -> Option<&'static str> {
    c.approximate
        .iter()
        .find(|(c, _)| c == id)
        .map(|(_, why)| *why)
}

// Lost Spirit: Deathrattle: give your minions +1 Attack (golden +2).

#[test]
fn lost_spirit_gives_the_minions_left_attack_when_it_dies() {
    for (id, more) in [("BG26_GIL_513", 1), ("BG26_GIL_513_G", 2)] {
        let other = Minion::new("BGS_034", 2, 5).tier(1);
        with(
            vec![lost_spirit(id, 1, 1), plain(1, 5), other],
            target(),
            |c| {
                kill(c, &[0]);
                assert_eq!(stats(c, 0), vec![(1 + more, 5), (2 + more, 5)], "{id}");
            },
        );
        assert_eq!(reason(&coverage(vec![lost_spirit(id, 1, 1)]), id), None);
    }
}

// Bassgill: Deathrattle: summon the highest-Health Murloc from your hand for
// this combat only (golden: the 2 highest). The hand is not seen.

#[test]
fn bassgill_is_a_hand_summoner_whose_summon_is_left_out() {
    for id in ["BG26_350", "BG26_350_G"] {
        assert!(crate::cards::HAND_SUMMONERS.contains(&id), "{id}");
        let bassgill = Minion::new(id, 7, 4).tribe(Tribes::MURLOC).tier(5);
        with(vec![bassgill.clone(), plain(1, 5)], target(), |c| {
            kill(c, &[0]);
            assert_eq!(ids(c, 0), vec!["BGS_119"], "{id}: nothing is summoned");
        });
        let why = reason(&coverage(vec![bassgill]), id);
        assert_eq!(why, Some(crate::cards_round5::FROM_HAND), "{id}");
    }
}

// Sr. Tomb Diver: Taunt. Deathrattle: make your right-most minion Golden
// (golden: the two right-most).

fn tomb_diver(id: &str) -> Minion {
    Minion::new(id, 4, 4).with(Keywords::TAUNT).tier(5)
}

#[test]
fn sr_tomb_diver_makes_the_rightmost_minion_its_golden_card_with_its_buffs() {
    // A 3/3 Lost Spirit (printed 1/1) becomes a 4/4 Golden Lost Spirit,
    // whose Deathrattle then gives +2 Attack.
    let diver = tomb_diver("TB_BaconShop_HERO_41_Buddy");
    with(
        vec![diver, plain(1, 5), lost_spirit("BG26_GIL_513", 3, 3)],
        target(),
        |c| {
            kill(c, &[0]);
            assert_eq!(ids(c, 0), vec!["BGS_119", "BG26_GIL_513_G"]);
            assert_eq!(stats(c, 0), vec![(1, 5), (4, 4)]);
            kill(c, &[1]);
            assert_eq!(stats(c, 0), vec![(3, 5)]);
        },
    );
}

#[test]
fn golden_sr_tomb_diver_passes_over_a_golden_minion() {
    // Right to left: the plain minion has no modelled golden card and stays
    // as it is, but counts; the Golden Spirit is passed over; the Spirit is
    // made Golden.
    let diver = tomb_diver("TB_BaconShop_HERO_41_Buddy_G");
    let board = vec![
        diver,
        lost_spirit("BG26_GIL_513", 2, 2),
        lost_spirit("BG26_GIL_513_G", 2, 2),
        plain(1, 5),
    ];
    with(board, target(), |c| {
        kill(c, &[0]);
        let golden = "BG26_GIL_513_G";
        assert_eq!(ids(c, 0), vec![golden, golden, "BGS_119"]);
        assert_eq!(stats(c, 0), vec![(3, 3), (2, 2), (1, 5)]);
    });
}

#[test]
fn a_minion_made_golden_without_printed_stats_keeps_its_stats() {
    let diver = tomb_diver("TB_BaconShop_HERO_41_Buddy");
    let spirit = Minion::new("BG26_GIL_513", 3, 3).tier(1);
    with(vec![diver, spirit], target(), |c| {
        kill(c, &[0]);
        assert_eq!(ids(c, 0), vec!["BG26_GIL_513_G"]);
        assert_eq!(stats(c, 0), vec![(3, 3)]);
    });
}

#[test]
fn a_summoned_token_made_golden_plays_its_golden_card() {
    // Sewer Lord's Rats are 3/2 tokens; the right-most becomes a 6/4 Golden
    // Rat, whose Deathrattle summons the 4/6 golden Turtle.
    let lord = Minion::new("BG35_604", 4, 6).tribe(Tribes::BEAST).tier(5);
    let diver = tomb_diver("TB_BaconShop_HERO_41_Buddy");
    with(vec![diver, lord], target(), |c| {
        kill(c, &[1]);
        kill(c, &[0]);
        assert_eq!(ids(c, 0), vec!["BG19_010", "BG19_010_G"]);
        assert_eq!(stats(c, 0), vec![(3, 2), (6, 4)]);
        kill(c, &[1]);
        assert_eq!(ids(c, 0), vec!["BG19_010", "BG19_010_Gt"]);
        assert_eq!(stats(c, 0)[1], (4, 6));
    });
}

#[test]
fn golden_cards_are_entered_only_when_a_card_makes_minions_golden() {
    let spirit = lost_spirit("BG26_GIL_513", 1, 1);
    with(vec![spirit.clone()], target(), |c| {
        assert!(c.p.gilded.is_empty())
    });
    let diver = tomb_diver("TB_BaconShop_HERO_41_Buddy");
    with(vec![diver, spirit], target(), |c| {
        assert!(!c.p.gilded.is_empty())
    });
}

#[test]
fn sr_tomb_diver_is_approximate_in_both_forms() {
    for id in ["TB_BaconShop_HERO_41_Buddy", "TB_BaconShop_HERO_41_Buddy_G"] {
        let why = reason(&coverage(vec![tomb_diver(id), plain(1, 1)]), id);
        assert!(why.is_some_and(|w| w.contains("made Golden")), "{id}");
    }
}

// Brann's Epic Egg: Taunt. Deathrattle: summon and get a random Battlecry
// minion (golden: 2). No Battlecry pool: the summon is left out.

#[test]
fn branns_epic_egg_leaves_its_random_battlecry_minion_out() {
    for id in ["TB_BaconShop_HERO_43_Buddy", "TB_BaconShop_HERO_43_Buddy_G"] {
        let egg = Minion::new(id, 0, 3).with(Keywords::TAUNT).tier(3);
        with(vec![egg.clone(), plain(1, 5)], target(), |c| {
            kill(c, &[0]);
            assert_eq!(ids(c, 0), vec!["BGS_119"], "{id}");
        });
        let why = reason(&coverage(vec![egg]), id);
        assert!(why.is_some_and(|w| w.contains("Battlecry")), "{id}");
    }
}

// Timewarped Nalaa: whenever you cast a spell, give a friendly minion of
// each type +4/+3 (golden +8/+6).

#[test]
fn timewarped_nalaa_gives_one_minion_of_each_type_stats_after_a_spell() {
    use super::Source;
    for (id, (attack, health)) in [("BG34_Giant_205", (4, 3)), ("BG34_Giant_205_G", (8, 6))] {
        let nalaa = Minion::new(id, 2, 2).tier(4);
        let beast = plain(1, 1).tribe(Tribes::BEAST);
        let murloc = Minion::new("BGS_034", 1, 1).tribe(Tribes::MURLOC).tier(1);
        with(vec![nalaa, beast, murloc], target(), |c| {
            let src = Source::Unit(uid_at(c, 0, 0));
            c.cast_spell(0, src, None, None, &[]);
            let grown = (1 + attack, 1 + health);
            assert_eq!(stats(c, 0), vec![(2, 2), grown, grown], "{id}");
        });
    }
}

#[test]
fn timewarped_nalaa_buffs_one_minion_of_a_type_per_spell() {
    // Runic Arcanist casts Shiny Ring (+1/+1) twice: Nalaa follows each, on
    // one of the two Beasts (the same card, so both are Beasts).
    let arcanist = Minion::new("BG36_245", 1, 1).tier(4);
    let nalaa = Minion::new("BG34_Giant_205", 2, 2).tier(4);
    let board = vec![
        arcanist,
        nalaa,
        plain(1, 1).tribe(Tribes::BEAST),
        plain(1, 1),
    ];
    with(board, target(), |c| {
        c.start_of_combat(0, [true, true]);
        let s = stats(c, 0);
        assert_eq!(&s[..2], &[(3, 3), (4, 4)]);
        let gained: i32 = s[2..].iter().map(|(a, _)| a - 3).sum();
        assert_eq!(gained, 8, "one Beast each time: {s:?}");
    });
}

#[test]
fn timewarped_nalaa_is_approximate_only_beside_a_spell_the_engine_does_not_play() {
    for id in ["BG34_Giant_205", "BG34_Giant_205_G"] {
        let nalaa = Minion::new(id, 2, 2).tier(4);
        assert_eq!(reason(&coverage(vec![nalaa.clone()]), id), None, "{id}");
        let recruiter = Minion::new("BG34_925", 2, 4).tribe(Tribes::NAGA).tier(2);
        let why = reason(&coverage(vec![nalaa, recruiter]), id);
        assert_eq!(why, Some(crate::cards_round8::UNPLAYED_SPELLS), "{id}");
    }
}
