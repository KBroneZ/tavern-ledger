//! The counters stay steady through a combat (T-611). On the user's logs
//! (build 253216) the game zeroes the local player's tags for the whole
//! combat packet and restores them at its end, and the Undead, Eternal
//! Knights and Beetle enchantments are only shown, in play, as copies for the
//! fight (the shop-turn originals are logged as unknown entities). Read at
//! every line, a naive reading blinks; these tests feed the combat line by
//! line and check every reading.

use bg_parser::counters::{Counter, CounterKind};
use bg_parser::LogReader;

mod counters_common;
use counters_common::{line, start, tag};

/// The player's tag values, set in the shop.
fn shop_values(spell: (i64, i64), gem: (i64, i64)) -> Vec<String> {
    vec![
        tag("2", "TAVERN_SPELL_ATTACK_INCREASE", &spell.0.to_string()),
        tag("2", "TAVERN_SPELL_HEALTH_INCREASE", &spell.1.to_string()),
        tag("2", "BACON_BLOODGEMBUFFATKVALUE", &gem.0.to_string()),
        tag("2", "BACON_BLOODGEMBUFFHEALTHVALUE", &gem.1.to_string()),
    ]
}

/// What the game does to the player's tags at the start of a combat.
fn wipe() -> Vec<String> {
    [
        "RESOURCES",
        "PLAYER_TECH_LEVEL",
        "TAVERN_SPELL_ATTACK_INCREASE",
        "TAVERN_SPELL_HEALTH_INCREASE",
        "BACON_BLOODGEMBUFFATKVALUE",
        "BACON_BLOODGEMBUFFHEALTHVALUE",
    ]
    .iter()
    .map(|name| tag("2", name, "0"))
    .collect()
}

/// A combat copy of a player enchantment, the way the game writes it: created
/// empty, shown with its card id while set aside, moved to play, then its
/// number changed in place. [`hide`] moves it away at the end of the combat.
fn combat_copy(id: i64, card: &str, numbers: &[i64]) -> Vec<String> {
    let mut lines = vec![
        line(1, &format!("FULL_ENTITY - Creating ID={id} CardID=")),
        line(2, "tag=ZONE value=SETASIDE"),
        line(2, &format!("tag=ENTITY_ID value={id}")),
        line(
            1,
            &format!("SHOW_ENTITY - Updating Entity={id} CardID={card}"),
        ),
        line(2, "tag=CONTROLLER value=2"),
        line(2, "tag=CARDTYPE value=ENCHANTMENT"),
        line(2, "tag=ATTACHED value=2"),
        line(2, "tag=ZONE value=SETASIDE"),
        line(2, &format!("tag=ENTITY_ID value={id}")),
        tag(&id.to_string(), "ZONE", "PLAY"),
    ];
    for n in numbers {
        lines.push(tag(
            &id.to_string(),
            "TAG_SCRIPT_DATA_NUM_1",
            &n.to_string(),
        ));
    }
    lines
}

fn hide(id: i64) -> Vec<String> {
    vec![tag(&id.to_string(), "ZONE", "SETASIDE")]
}

/// Feeds `lines` one by one and gives the reading after each.
fn readings(reader: &mut LogReader, lines: &[String]) -> Vec<Option<Vec<Counter>>> {
    lines
        .iter()
        .map(|l| {
            reader.feed(l);
            reader.counters_now()
        })
        .collect()
}

/// A game in its first shop turn with `parts` played in it; a closing line
/// ends the last packet.
fn game_in_the_shop(parts: &[Vec<String>]) -> LogReader {
    let mut reader = LogReader::default();
    start().iter().for_each(|l| reader.feed(l));
    parts.iter().flatten().for_each(|l| reader.feed(l));
    reader.feed(&tag("2", "RESOURCES", "1"));
    reader
}

fn spell(atk: i64, health: i64) -> Counter {
    Counter::of_stats(CounterKind::TavernSpell, atk, health)
}

fn turn(n: i64) -> String {
    tag("GameEntity", "TURN", &n.to_string())
}

#[test]
fn the_tag_counters_hold_through_the_wipe_and_the_restore() {
    let mut reader = game_in_the_shop(&[shop_values((2, 5), (3, 1))]);
    let before = reader.counters_now().expect("known");
    assert_eq!(
        before,
        [Counter::of_stats(CounterKind::BloodGem, 4, 2), spell(2, 5)]
    );
    let mut combat = vec![turn(2)];
    combat.extend(wipe());
    combat.extend(shop_values((2, 5), (3, 1)));
    combat.push(tag("2", "PLAYER_TECH_LEVEL", "4"));
    combat.push(turn(3));
    for (i, now) in readings(&mut reader, &combat).into_iter().enumerate() {
        assert_eq!(now.as_ref(), Some(&before), "after line {i}");
    }
}

#[test]
fn values_changed_during_the_combat_show_when_the_next_shop_turn_starts() {
    let mut reader = game_in_the_shop(&[shop_values((2, 5), (0, 0))]);
    let mut combat = vec![turn(2)];
    combat.extend(wipe());
    combat.push(tag("2", "TAVERN_SPELL_ATTACK_INCREASE", "3"));
    combat.push(tag("2", "TAVERN_SPELL_HEALTH_INCREASE", "5"));
    combat.push(tag("2", "PLAYER_TECH_LEVEL", "4"));
    readings(&mut reader, &combat);
    assert_eq!(
        reader.counters_now(),
        Some(vec![spell(2, 5)]),
        "still the shop's"
    );
    reader.feed(&turn(3));
    assert_eq!(reader.counters_now(), Some(vec![spell(3, 5)]));
}

#[test]
fn the_tag_counters_follow_the_shop_turn_again_after_a_combat() {
    let mut reader = game_in_the_shop(&[shop_values((2, 5), (0, 0))]);
    let mut combat = vec![turn(2)];
    combat.extend(wipe());
    combat.extend(shop_values((2, 5), (0, 0)));
    combat.push(turn(3));
    readings(&mut reader, &combat);
    reader.feed(&tag("2", "TAVERN_SPELL_ATTACK_INCREASE", "4"));
    assert_eq!(reader.counters_now(), Some(vec![spell(4, 5)]));
}

#[test]
fn the_enchantment_counters_appear_with_the_first_combat_and_stay_after_it() {
    let mut reader = game_in_the_shop(&[]);
    assert_eq!(
        reader.counters_now(),
        Some(vec![]),
        "nothing shown in the shop"
    );

    let mut combat = vec![turn(2)];
    combat.extend(wipe());
    combat.extend(combat_copy(7421, "BG25_011pe", &[33]));
    combat.extend(combat_copy(7733, "BG25_008pe", &[1, 2, 3, 4, 5]));
    combat.extend(combat_copy(7800, "BG31_808pe", &[]));
    combat.push(tag("7800", "TAG_SCRIPT_DATA_NUM_1", "6"));
    combat.push(tag("7800", "TAG_SCRIPT_DATA_NUM_2", "6"));
    combat.extend(hide(7421));
    combat.extend(hide(7733));
    combat.extend(hide(7800));
    combat.push(tag("2", "PLAYER_TECH_LEVEL", "4"));
    combat.push(turn(3));
    let all = readings(&mut reader, &combat);

    // Once a kind is shown it is never dropped again, at any line.
    let mut shown: Vec<CounterKind> = Vec::new();
    for (i, now) in all.iter().enumerate() {
        let kinds: Vec<CounterKind> = now
            .as_ref()
            .expect("known")
            .iter()
            .map(|c| c.kind)
            .collect();
        for kind in &shown {
            assert!(kinds.contains(kind), "{kind:?} dropped after line {i}");
        }
        shown = kinds;
    }
    let last = all.last().cloned().flatten().expect("known");
    assert_eq!(
        last,
        [
            Counter::of_attack(CounterKind::UndeadAttack, 33),
            Counter::of_count(CounterKind::EternalKnights, 5),
            Counter::of_stats(CounterKind::Beetle, 6, 6),
        ]
    );
}

#[test]
fn a_new_combat_copy_that_starts_empty_does_not_drop_the_total() {
    let mut reader = game_in_the_shop(&[]);
    let mut first = vec![turn(2)];
    first.extend(combat_copy(7421, "BG25_011pe", &[33]));
    first.extend(hide(7421));
    first.push(turn(3));
    readings(&mut reader, &first);

    // The next combat's copy is created with no number yet.
    let mut second = vec![turn(4)];
    second.extend(combat_copy(7900, "BG25_011pe", &[]));
    second.push(tag("7900", "TAG_SCRIPT_DATA_NUM_1", "35"));
    for (i, now) in readings(&mut reader, &second).into_iter().enumerate() {
        let attack = now.expect("known")[0].atk.expect("an attack");
        assert!(attack >= 33, "dropped to {attack} after line {i}");
    }
    assert_eq!(
        reader.counters_now(),
        Some(vec![Counter::of_attack(CounterKind::UndeadAttack, 35)])
    );
}

#[test]
fn a_copy_of_the_opponents_or_the_teammates_never_updates_the_memory() {
    let mut reader = game_in_the_shop(&[shop_values((1, 1), (0, 0))]);
    let mut combat = vec![turn(2)];
    combat.extend(wipe());
    for (id, owner) in [(7421, 3), (7422, 5)] {
        let mut copy = combat_copy(id, "BG25_011pe", &[50]);
        copy[6] = line(2, &format!("tag=ATTACHED value={owner}"));
        combat.extend(copy);
    }
    combat.extend(shop_values((1, 1), (0, 0)));
    combat.push(turn(3));
    readings(&mut reader, &combat);
    assert_eq!(reader.counters_now(), Some(vec![spell(1, 1)]));
}

#[test]
fn a_second_combat_copy_filled_in_two_lines_never_lowers_either_number() {
    let mut reader = game_in_the_shop(&[]);
    let mut first = vec![turn(2)];
    first.extend(combat_copy(7800, "BG31_808pe", &[]));
    first.push(tag("7800", "TAG_SCRIPT_DATA_NUM_1", "6"));
    first.push(tag("7800", "TAG_SCRIPT_DATA_NUM_2", "6"));
    first.extend(hide(7800));
    first.push(turn(3));
    readings(&mut reader, &first);

    let mut second = vec![turn(4)];
    second.extend(combat_copy(7900, "BG31_808pe", &[]));
    second.push(tag("7900", "TAG_SCRIPT_DATA_NUM_1", "8"));
    second.push(tag("7900", "TAG_SCRIPT_DATA_NUM_2", "8"));
    for (i, now) in readings(&mut reader, &second).into_iter().enumerate() {
        let beetle = now.expect("known")[0].clone();
        let (atk, health) = (beetle.atk.expect("atk"), beetle.health.expect("health"));
        assert!(atk >= 6 && health >= 6, "({atk}, {health}) after line {i}");
    }
    assert_eq!(
        reader.counters_now(),
        Some(vec![Counter::of_stats(CounterKind::Beetle, 8, 8)])
    );
}

/// The start of a game joined after it began (a reconnect): the game entity
/// already has its turn and the player entity its values.
fn joined_in_turn(turn: i64) -> Vec<String> {
    vec![
        line(0, "CREATE_GAME"),
        line(1, "GameEntity EntityID=1"),
        line(2, "tag=CARDTYPE value=GAME"),
        line(2, &format!("tag=TURN value={turn}")),
        line(2, "tag=ENTITY_ID value=1"),
        line(1, "Player EntityID=2 PlayerID=2 GameAccountId=[hi=1 lo=1]"),
        line(2, "tag=PLAYER_ID value=2"),
        line(2, "tag=CARDTYPE value=PLAYER"),
        line(2, "tag=BACON_BLOODGEMBUFFATKVALUE value=3"),
        line(2, "tag=ENTITY_ID value=2"),
        "D 12:00:00.0000000 GameState.DebugPrintGame() - BuildNumber=253216".into(),
        "D 12:00:00.0000000 GameState.DebugPrintGame() - GameType=GT_BATTLEGROUNDS".into(),
        tag("2", "RESOURCES", "1"),
    ]
}

#[test]
fn a_game_joined_in_a_shop_turn_has_the_players_values_at_once() {
    let mut reader = LogReader::default();
    joined_in_turn(7).iter().for_each(|l| reader.feed(l));
    assert_eq!(
        reader.counters_now(),
        Some(vec![Counter::of_stats(CounterKind::BloodGem, 4, 1)])
    );
}

#[test]
fn a_game_joined_in_a_combat_does_not_read_the_wiped_player() {
    let mut reader = LogReader::default();
    joined_in_turn(8).iter().for_each(|l| reader.feed(l));
    assert_eq!(reader.counters_now(), Some(vec![]));
}
