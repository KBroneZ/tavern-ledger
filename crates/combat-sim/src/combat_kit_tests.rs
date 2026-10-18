//! Hero powers and trinkets played at the start of combat (T-628 round 2),
//! on small boards whose outcome is certain.

use super::{Combat, Prepared};
use crate::model::{Keywords, Kit, KitCard, Minion, Tribes, Warband};
use crate::rng::Rng;

/// A keyword-only card, so the boards are fully modelled.
fn plain(attack: i32, health: i32) -> Minion {
    Minion::new("BGS_119", attack, health).tier(1)
}

/// Keyword-only cards of their own, for a minion type: the engine keeps a
/// card's types per card id.
fn of_type(id: &str, tribe: Tribes, attack: i32, health: i32) -> Minion {
    Minion::new(id, attack, health).tribe(tribe).tier(1)
}

fn card(id: &str, counters: [i32; 5]) -> KitCard {
    KitCard {
        counters,
        ..KitCard::new(id)
    }
}

fn band(minions: Vec<Minion>, powers: Vec<KitCard>, trinkets: Vec<KitCard>) -> Warband {
    Warband {
        kit: Some(Kit { powers, trinkets }),
        tavern_tier: Some(1),
        ..Warband::new(minions)
    }
}

/// Plays the player's start of combat (the player first) and hands the
/// combat over.
fn after_start<R>(player: Warband, opponent: Vec<Minion>, f: impl FnOnce(&Combat) -> R) -> R {
    let opponent = band(opponent, Vec::new(), Vec::new());
    let p = Prepared::new(&player, &opponent, true);
    let mut rng = Rng::new(11);
    let mut c = Combat::new(&p, &mut rng);
    c.start_of_combat(0, [true, true]);
    f(&c)
}

fn stats(c: &Combat, side: usize) -> Vec<(i32, i32)> {
    c.sides[side]
        .units
        .iter()
        .map(|u| (u.attack, u.health))
        .collect()
}

fn keywords(c: &Combat, side: usize) -> Vec<Keywords> {
    c.sides[side].units.iter().map(|u| u.kw).collect()
}

const NONE: [i32; 5] = [0; 5];
const SHIELD: Keywords = Keywords::DIVINE_SHIELD;

#[test]
fn wax_warband_gives_one_minion_of_each_type_its_third_number() {
    // Two Beasts and a Mech: one Beast and the Mech gain +4/+4.
    let minions = vec![
        of_type("BG28_603t", Tribes::BEAST, 1, 1),
        of_type("BG28_603t", Tribes::BEAST, 1, 1),
        of_type("BG_BOT_911", Tribes::MECH, 1, 1),
    ];
    let wax = card("TB_BaconShop_HP_037a", [9, 0, 4, 0, 0]);
    let player = band(minions, vec![wax], Vec::new());
    after_start(player, vec![plain(0, 9)], |c| {
        let mut s = stats(c, 0);
        assert_eq!(s.pop(), Some((5, 5)));
        s.sort_unstable();
        assert_eq!(s, [(1, 1), (5, 5)]);
    });
}

#[test]
fn swatting_insects_and_water_invocation_pick_an_end() {
    let insects = card("TB_BaconShop_HP_086", NONE);
    let player = band(vec![plain(1, 1), plain(1, 1)], vec![insects], Vec::new());
    after_start(player, vec![plain(0, 9)], |c| {
        let all = Keywords::WINDFURY.union(SHIELD).union(Keywords::TAUNT);
        assert_eq!(keywords(c, 0), [all, Keywords::NONE]);
    });
    let water = card("BG22_HERO_001p_t3", NONE);
    let player = band(vec![plain(1, 1), plain(1, 1)], vec![water], Vec::new());
    after_start(player, vec![plain(0, 9)], |c| {
        assert_eq!(
            keywords(c, 0),
            [Keywords::NONE, SHIELD.union(Keywords::TAUNT)]
        );
    });
}

#[test]
fn holy_mallet_shields_both_ends_or_the_one_minion() {
    let mallet = || card("BG30_MagicItem_902", NONE);
    let player = band(
        vec![plain(1, 1), plain(1, 1), plain(1, 1)],
        Vec::new(),
        vec![mallet()],
    );
    after_start(player, vec![plain(0, 9)], |c| {
        assert_eq!(keywords(c, 0), [SHIELD, Keywords::NONE, SHIELD]);
    });
    let player = band(vec![plain(1, 1)], Vec::new(), vec![mallet()]);
    after_start(player, vec![plain(0, 9)], |c| {
        assert_eq!(keywords(c, 0), [SHIELD])
    });
}

#[test]
fn fire_invocation_doubles_the_left_most_attack() {
    let fire = card("BG22_HERO_001p_t2", NONE);
    let player = band(vec![plain(3, 2), plain(3, 2)], vec![fire], Vec::new());
    after_start(player, vec![plain(0, 9)], |c| {
        assert_eq!(stats(c, 0), [(6, 2), (3, 2)]);
    });
}

#[test]
fn lightning_invocation_deals_five_damage_among_the_enemies() {
    let lightning = card("BG22_HERO_001p_t4", NONE);
    let player = band(vec![plain(1, 1)], vec![lightning], Vec::new());
    after_start(player, vec![plain(0, 20), plain(0, 20)], |c| {
        let left: i32 = stats(c, 1).iter().map(|(_, h)| *h).sum();
        assert_eq!(left, 40 - 5);
    });
}

#[test]
fn bronze_timepiece_adds_half_the_attack_rounded_down() {
    let timepiece = card("BG30_MagicItem_995", NONE);
    let player = band(vec![plain(5, 1), plain(4, 1)], Vec::new(), vec![timepiece]);
    after_start(player, vec![plain(0, 9)], |c| {
        assert_eq!(stats(c, 0), [(5, 3), (4, 3)]);
    });
}

#[test]
fn training_certificate_doubles_the_two_lowest_attack_minions() {
    // The left-most of the two 2-Attack minions and the 1-Attack one.
    let certificate = card("BG30_MagicItem_962", NONE);
    let minions = vec![plain(5, 5), plain(2, 3), plain(1, 4), plain(2, 6)];
    let player = band(minions, Vec::new(), vec![certificate]);
    after_start(player, vec![plain(0, 9)], |c| {
        assert_eq!(stats(c, 0), [(5, 5), (4, 6), (2, 8), (2, 6)]);
    });
}

#[test]
fn converter_portrait_gives_faceless_converters_reborn() {
    let portrait = card("BG36_MagicItem_600", NONE);
    let converter = Minion::new("BG36_318", 2, 2).tier(1);
    let player = band(vec![converter, plain(1, 1)], Vec::new(), vec![portrait]);
    after_start(player, vec![plain(0, 9)], |c| {
        assert_eq!(keywords(c, 0), [Keywords::REBORN, Keywords::NONE]);
    });
}

#[test]
fn protective_ring_shields_up_to_four_pirates() {
    let ring = card("BG35_MagicItem_711", NONE);
    let pirate = || of_type("BG26_361", Tribes::PIRATE, 1, 1);
    let minions = vec![pirate(), plain(1, 1), pirate(), pirate()];
    let player = band(minions, Vec::new(), vec![ring]);
    after_start(player, vec![plain(0, 9)], |c| {
        assert_eq!(keywords(c, 0), [SHIELD, Keywords::NONE, SHIELD, SHIELD]);
    });
}

#[test]
fn herald_sticker_triggers_every_deathrattle() {
    // Bonehead: Deathrattle: summon two 1/1 Skeletons.
    let sticker = card("BG32_MagicItem_306", NONE);
    let bonehead = Minion::new("BG28_300", 1, 1).tribe(Tribes::UNDEAD).tier(1);
    let player = band(vec![bonehead], Vec::new(), vec![sticker]);
    after_start(player, vec![plain(0, 9)], |c| {
        assert_eq!(c.sides[0].units.len(), 3)
    });
}

#[test]
fn trinkets_come_before_hero_powers() {
    // Training Certificate doubles the 1/1 to 2/2, then Wax Warband gives
    // +3/+3: 5/5. The other way round it would be 8/8.
    let certificate = card("BG30_MagicItem_962", NONE);
    let wax = card("TB_BaconShop_HP_037a", [9, 0, 3, 0, 0]);
    let player = band(
        vec![of_type("BG28_603t", Tribes::BEAST, 1, 1)],
        vec![wax],
        vec![certificate],
    );
    after_start(player, vec![plain(0, 9)], |c| {
        assert_eq!(stats(c, 0), [(5, 5)])
    });
}

#[test]
fn a_board_taken_after_start_of_combat_plays_none_of_it() {
    let mallet = card("BG30_MagicItem_902", NONE);
    let player = band(vec![plain(1, 1)], Vec::new(), vec![mallet]);
    let opponent = band(vec![plain(0, 9)], Vec::new(), Vec::new());
    let p = Prepared::new(&player, &opponent, false);
    let mut rng = Rng::new(11);
    let mut c = Combat::new(&p, &mut rng);
    c.open();
    assert_eq!(keywords(&c, 0), [Keywords::NONE]);
}

/// Plays every start of combat of both boards (the player first).
fn both_start<R>(player: Vec<Minion>, opponent: Vec<Minion>, f: impl FnOnce(&Combat) -> R) -> R {
    after_start(band(player, Vec::new(), Vec::new()), opponent, f)
}

#[test]
fn amber_guardian_gives_another_dragon_stats_and_a_shield() {
    let dragon = || of_type("BG25_001", Tribes::DRAGON, 1, 1);
    let amber = Minion::new("BG24_500", 2, 5).tribe(Tribes::DRAGON).tier(3);
    both_start(vec![amber, dragon()], vec![plain(0, 9)], |c| {
        assert_eq!(stats(c, 0), [(2, 5), (3, 3)]);
        assert_eq!(keywords(c, 0)[1], SHIELD);
    });
}

#[test]
fn runic_arcanist_casts_shiny_ring_twice() {
    let arcanist = Minion::new("BG36_245", 2, 4).tier(4);
    both_start(vec![arcanist, plain(1, 1)], vec![plain(0, 9)], |c| {
        assert_eq!(stats(c, 0), [(4, 6), (3, 3)]);
    });
}

#[test]
fn boom_in_a_box_hits_every_other_minion() {
    let boom = Minion::new("BG36_620", 2, 6).tier(5);
    both_start(
        vec![boom, plain(1, 5)],
        vec![plain(1, 2), plain(1, 9)],
        |c| {
            // The 1/2 dies; the others take 3.
            assert_eq!(stats(c, 0), [(2, 6), (1, 2)]);
            assert_eq!(stats(c, 1), [(1, 6)]);
        },
    );
}
