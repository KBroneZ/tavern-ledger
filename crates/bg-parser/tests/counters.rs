//! The player's own game-long counters, live (T-611): what the overlay shows
//! as running totals. Synthetic log shaped like the user's, written out below:
//! the local player is entity 2, the opponent's dummy player entity 3, and
//! entity 5 stands for a Duos teammate's hero.

use bg_parser::counters::{counters_from, Counter, CounterInputs, CounterKind, ENABLED_KINDS};
use bg_parser::report::CombatValues;
use bg_parser::LogReader;

const T: &str = "D 12:00:00.0000000 GameState.DebugPrintPower() - ";

fn line(indent: usize, body: &str) -> String {
    format!("{T}{}{body}", "    ".repeat(indent))
}

fn tag(entity: &str, tag: &str, value: &str) -> String {
    line(
        1,
        &format!("TAG_CHANGE Entity={entity} tag={tag} value={value} "),
    )
}

/// An enchantment as the game creates it: one `FULL_ENTITY` packet.
fn enchantment(id: i64, card: &str, attached: i64, zone: &str, nums: (i64, i64)) -> Vec<String> {
    vec![
        line(1, &format!("FULL_ENTITY - Creating ID={id} CardID={card}")),
        line(2, "tag=CARDTYPE value=ENCHANTMENT"),
        line(2, &format!("tag=ATTACHED value={attached}")),
        line(2, &format!("tag=ZONE value={zone}")),
        line(2, &format!("tag=TAG_SCRIPT_DATA_NUM_1 value={}", nums.0)),
        line(2, &format!("tag=TAG_SCRIPT_DATA_NUM_2 value={}", nums.1)),
        line(2, &format!("tag=ENTITY_ID value={id}")),
    ]
}

/// A Solo game in its first shop turn: no values yet.
fn start() -> Vec<String> {
    vec![
        line(0, "CREATE_GAME"),
        line(1, "GameEntity EntityID=1"),
        line(2, "tag=CARDTYPE value=GAME"),
        line(2, "tag=ENTITY_ID value=1"),
        line(1, "Player EntityID=2 PlayerID=2 GameAccountId=[hi=1 lo=1]"),
        line(2, "tag=PLAYER_ID value=2"),
        line(2, "tag=CARDTYPE value=PLAYER"),
        line(2, "tag=HERO_ENTITY value=10"),
        line(2, "tag=ENTITY_ID value=2"),
        line(1, "Player EntityID=3 PlayerID=10 GameAccountId=[hi=0 lo=0]"),
        line(2, "tag=PLAYER_ID value=10"),
        line(2, "tag=CARDTYPE value=PLAYER"),
        line(2, "tag=HERO_ENTITY value=11"),
        line(2, "tag=BACON_DUMMY_PLAYER value=1"),
        line(2, "tag=ENTITY_ID value=3"),
        "D 12:00:00.0000000 GameState.DebugPrintGame() - BuildNumber=253216".into(),
        "D 12:00:00.0000000 GameState.DebugPrintGame() - GameType=GT_BATTLEGROUNDS".into(),
        line(1, "FULL_ENTITY - Creating ID=5 CardID=SYN_TEAMMATE_HERO"),
        line(2, "tag=CARDTYPE value=HERO"),
        line(2, "tag=ENTITY_ID value=5"),
        tag("GameEntity", "TURN", "1"),
    ]
}

fn reader_with(parts: &[Vec<String>]) -> LogReader {
    let mut reader = LogReader::default();
    start().iter().for_each(|l| reader.feed(l));
    parts.iter().flatten().for_each(|l| reader.feed(l));
    // Any later line closes the last packet, as in the live log.
    reader.feed(&tag("2", "RESOURCES", "1"));
    reader
}

fn counters(parts: &[Vec<String>]) -> Option<Vec<Counter>> {
    reader_with(parts).counters_now()
}

fn one(lines: Vec<String>) -> Vec<Vec<String>> {
    vec![lines]
}

#[test]
fn a_game_with_nothing_to_count_gives_an_empty_list_not_unknown() {
    assert_eq!(counters(&[]), Some(vec![]));
}

#[test]
fn blood_gem_carries_the_totals_one_plus_the_extra() {
    let values = one(vec![
        tag("2", "BACON_BLOODGEMBUFFATKVALUE", "3"),
        tag("2", "BACON_BLOODGEMBUFFHEALTHVALUE", "2"),
    ]);
    assert_eq!(
        counters(&values),
        Some(vec![Counter::of_stats(CounterKind::BloodGem, 4, 3)])
    );
    let only_health = one(vec![tag("2", "BACON_BLOODGEMBUFFHEALTHVALUE", "2")]);
    assert_eq!(
        counters(&only_health),
        Some(vec![Counter::of_stats(CounterKind::BloodGem, 1, 3)]),
        "the other total is the gem's base"
    );
}

#[test]
fn blood_gem_is_listed_only_when_the_extra_is_above_zero() {
    let none = one(vec![
        tag("2", "BACON_BLOODGEMBUFFATKVALUE", "0"),
        tag("2", "BACON_BLOODGEMBUFFHEALTHVALUE", "0"),
    ]);
    assert_eq!(counters(&none), Some(vec![]), "the base +1/+1 alone");
}

#[test]
fn tavern_spell_carries_the_extra_attack_and_health() {
    let values = one(vec![
        tag("2", "TAVERN_SPELL_ATTACK_INCREASE", "2"),
        tag("2", "TAVERN_SPELL_HEALTH_INCREASE", "5"),
    ]);
    assert_eq!(
        counters(&values),
        Some(vec![Counter::of_stats(CounterKind::TavernSpell, 2, 5)])
    );
}

#[test]
fn a_spell_buff_of_zero_attack_and_five_health_is_kept() {
    let values = one(vec![tag("2", "TAVERN_SPELL_HEALTH_INCREASE", "5")]);
    assert_eq!(
        counters(&values),
        Some(vec![Counter::of_stats(CounterKind::TavernSpell, 0, 5)])
    );
}

#[test]
fn undead_attack_is_the_enchantments_first_number() {
    let values = one(enchantment(60, "BG25_011pe", 2, "PLAY", (7, 0)));
    assert_eq!(
        counters(&values),
        Some(vec![Counter::of_attack(CounterKind::UndeadAttack, 7)])
    );
}

#[test]
fn eternal_knights_count_the_fallen_ones() {
    let values = one(enchantment(62, "BG25_008pe", 2, "PLAY", (5, 0)));
    assert_eq!(
        counters(&values),
        Some(vec![Counter::of_count(CounterKind::EternalKnights, 5)])
    );
}

#[test]
fn beetles_carry_the_enchantments_two_numbers() {
    let values = one(enchantment(61, "BG31_808pe", 2, "PLAY", (12, 11)));
    assert_eq!(
        counters(&values),
        Some(vec![Counter::of_stats(CounterKind::Beetle, 12, 11)])
    );
}

#[test]
fn tavern_elementals_carry_the_player_enchantments_two_numbers() {
    let values = one(enchantment(70, "BG_ShopBuff_Elemental", 2, "PLAY", (4, 4)));
    assert_eq!(
        counters(&values),
        Some(vec![Counter::of_stats(CounterKind::ShopElemental, 4, 4)])
    );
}

#[test]
fn tavern_elementals_follow_the_enchantment_the_way_the_game_writes_it() {
    // The user's logs: the enchantment is created empty, shown with its card
    // id while set aside, moved to play, and then its numbers change in
    // place (1, 2, 3, 4), never as a new entity.
    let created = vec![
        line(1, "FULL_ENTITY - Creating ID=437 CardID="),
        line(2, "tag=ZONE value=SETASIDE"),
        line(2, "tag=ENTITY_ID value=437"),
        line(
            1,
            "SHOW_ENTITY - Updating Entity=437 CardID=BG_ShopBuff_Elemental",
        ),
        line(2, "tag=CONTROLLER value=2"),
        line(2, "tag=CARDTYPE value=ENCHANTMENT"),
        line(2, "tag=ATTACHED value=2"),
        line(2, "tag=ZONE value=SETASIDE"),
        line(2, "tag=ENTITY_ID value=437"),
        tag("437", "ZONE", "PLAY"),
        tag("437", "TAG_SCRIPT_DATA_NUM_1", "1"),
        tag("437", "TAG_SCRIPT_DATA_NUM_2", "1"),
    ];
    let mut reader = reader_with(&[created]);
    assert_eq!(
        reader.counters_now(),
        Some(vec![Counter::of_stats(CounterKind::ShopElemental, 1, 1)])
    );
    for total in ["2", "4"] {
        reader.feed(&tag("437", "TAG_SCRIPT_DATA_NUM_1", total));
        reader.feed(&tag("437", "TAG_SCRIPT_DATA_NUM_2", total));
    }
    assert_eq!(
        reader.counters_now(),
        Some(vec![Counter::of_stats(CounterKind::ShopElemental, 4, 4)])
    );
}

#[test]
fn tavern_elementals_that_are_not_in_play_or_empty_are_not_listed() {
    let set_aside = one(enchantment(
        70,
        "BG_ShopBuff_Elemental",
        2,
        "SETASIDE",
        (4, 4),
    ));
    assert_eq!(counters(&set_aside), Some(vec![]));
    let zero = one(enchantment(70, "BG_ShopBuff_Elemental", 2, "PLAY", (0, 0)));
    assert_eq!(counters(&zero), Some(vec![]));
}

#[test]
fn the_tavern_enchantments_that_are_not_totals_are_never_read() {
    // The log's other tavern buffs (per minion, or set again every event)
    // are not the player's current total: not counters (see counters-log).
    let mut parts = Vec::new();
    for (id, card) in [
        (80, "BG_ShopBuff"),
        (81, "BG_ShopBuff_Ench"),
        (82, "BG_ShopBuff_MultiRace_Ench"),
        (83, "BG_ShopBuff_Elemental_Ench"),
    ] {
        parts.push(enchantment(id, card, 2, "PLAY", (6, 6)));
    }
    parts.push(vec![
        tag("2", "BACON_PLAYER_EXTRA_GOLD_NEXT_TURN", "2"),
        tag("2", "BACON_FREE_REFRESH_COUNT", "3"),
    ]);
    assert_eq!(counters(&parts), Some(vec![]));
}

#[test]
fn an_enchantment_that_is_not_in_play_is_not_counted() {
    let values = one(enchantment(62, "BG25_008pe", 2, "SETASIDE", (9, 0)));
    assert_eq!(counters(&values), Some(vec![]));
}

#[test]
fn entries_come_in_the_fixed_order_of_the_kinds() {
    let mut parts = vec![vec![
        tag("2", "TAVERN_SPELL_ATTACK_INCREASE", "1"),
        tag("2", "BACON_BLOODGEMBUFFATKVALUE", "1"),
    ]];
    parts.push(enchantment(61, "BG31_808pe", 2, "PLAY", (3, 3)));
    parts.push(enchantment(62, "BG25_008pe", 2, "PLAY", (2, 0)));
    parts.push(enchantment(60, "BG25_011pe", 2, "PLAY", (4, 0)));
    parts.push(enchantment(70, "BG_ShopBuff_Elemental", 2, "PLAY", (2, 2)));
    let kinds: Vec<CounterKind> = counters(&parts)
        .expect("known")
        .iter()
        .map(|c| c.kind)
        .collect();
    assert_eq!(
        kinds,
        [
            CounterKind::BloodGem,
            CounterKind::TavernSpell,
            CounterKind::UndeadAttack,
            CounterKind::EternalKnights,
            CounterKind::Beetle,
            CounterKind::ShopElemental,
        ]
    );
}

#[test]
fn counters_are_given_in_a_combat_turn_too() {
    // No turn gate: the shop-only reads stay `None` in a combat turn, the
    // counters do not.
    let parts = vec![
        vec![tag("2", "TAVERN_SPELL_ATTACK_INCREASE", "2")],
        enchantment(62, "BG25_008pe", 2, "PLAY", (3, 0)),
        vec![tag("GameEntity", "TURN", "2")],
    ];
    let reader = reader_with(&parts);
    assert!(reader.own_board_now().is_none(), "a combat turn");
    assert_eq!(
        reader.counters_now(),
        Some(vec![
            Counter::of_stats(CounterKind::TavernSpell, 2, 0),
            Counter::of_count(CounterKind::EternalKnights, 3),
        ])
    );
}

#[test]
fn counters_follow_the_log_as_it_changes() {
    let mut reader = reader_with(&[vec![tag("2", "TAVERN_SPELL_ATTACK_INCREASE", "2")]]);
    reader.feed(&tag("2", "TAVERN_SPELL_ATTACK_INCREASE", "3"));
    assert_eq!(
        reader.counters_now(),
        Some(vec![Counter::of_stats(CounterKind::TavernSpell, 3, 0)])
    );
}

#[test]
fn no_game_or_a_game_that_is_over_is_unknown() {
    assert_eq!(LogReader::default().counters_now(), None);
    let ended = vec![
        vec![tag("2", "TAVERN_SPELL_ATTACK_INCREASE", "2")],
        vec![tag("GameEntity", "STATE", "COMPLETE")],
    ];
    assert_eq!(counters(&ended), None);
}

#[test]
fn a_game_whose_player_is_not_known_yet_is_unknown() {
    let mut reader = LogReader::default();
    reader.feed(&line(0, "CREATE_GAME"));
    reader.feed("D 12:00:00.0000000 GameState.DebugPrintGame() - GameType=GT_BATTLEGROUNDS");
    assert_eq!(reader.counters_now(), None);
}

#[test]
fn a_reader_error_is_unknown_never_an_empty_list() {
    let mut reader = reader_with(&[vec![tag("2", "TAVERN_SPELL_ATTACK_INCREASE", "2")]]);
    assert!(reader.counters_now().is_some());
    reader.feed("this line has no timestamp");
    assert_eq!(reader.counters_now(), None);
}

#[test]
fn an_opponents_and_a_teammates_values_never_appear() {
    let mut parts = vec![vec![
        tag("3", "BACON_BLOODGEMBUFFATKVALUE", "9"),
        tag("3", "TAVERN_SPELL_ATTACK_INCREASE", "9"),
        // A teammate's hero is not the player entity.
        tag("5", "BACON_BLOODGEMBUFFATKVALUE", "8"),
        tag("5", "TAVERN_SPELL_HEALTH_INCREASE", "8"),
    ]];
    for (id, card, owner) in [
        (60, "BG25_011pe", 3),
        (61, "BG31_808pe", 3),
        (62, "BG25_008pe", 3),
        (63, "BG25_011pe", 5),
        (64, "BG31_808pe", 5),
        (65, "BG25_008pe", 5),
        (66, "BG_ShopBuff_Elemental", 3),
        (67, "BG_ShopBuff_Elemental", 5),
    ] {
        parts.push(enchantment(id, card, owner, "PLAY", (6, 6)));
    }
    assert_eq!(counters(&parts), Some(vec![]));

    // The player's own are still read next to them.
    parts.push(vec![tag("2", "TAVERN_SPELL_ATTACK_INCREASE", "1")]);
    assert_eq!(
        counters(&parts),
        Some(vec![Counter::of_stats(CounterKind::TavernSpell, 1, 0)])
    );
}

#[test]
fn a_value_outside_the_range_is_passed_on_for_the_tracker_to_refuse() {
    let values = one(vec![tag("2", "TAVERN_SPELL_ATTACK_INCREASE", "100000")]);
    assert_eq!(
        counters(&values),
        Some(vec![Counter::of_stats(
            CounterKind::TavernSpell,
            100_000,
            0
        )]),
        "never clamped here"
    );
    let negative = one(vec![tag("2", "TAVERN_SPELL_HEALTH_INCREASE", "-4")]);
    assert_eq!(
        counters(&negative),
        Some(vec![Counter::of_stats(CounterKind::TavernSpell, 0, -4)])
    );
    let gem = one(vec![tag("2", "BACON_BLOODGEMBUFFHEALTHVALUE", "-1")]);
    let [counter] = counters(&gem).expect("known").try_into().expect("listed");
    assert!(
        !counter.in_range(),
        "a negative extra never reads as a total"
    );
}

#[test]
fn a_kind_outside_the_enabled_list_never_appears() {
    let values = CounterInputs {
        values: CombatValues {
            blood_gem: (3, 2),
            spell: (1, 1),
            knights_dead: 4,
            undead_attack: 5,
            beetle: (6, 6),
        },
        shop_elemental: (7, 7),
    };
    let all = counters_from(&values, ENABLED_KINDS);
    assert_eq!(
        all.len(),
        ENABLED_KINDS.len(),
        "every enabled kind has data"
    );

    let only = counters_from(&values, &[CounterKind::Beetle, CounterKind::BloodGem]);
    let kinds: Vec<_> = only.iter().map(|c| c.kind).collect();
    assert_eq!(
        kinds,
        [CounterKind::BloodGem, CounterKind::Beetle],
        "only the listed ones, in the kinds' order"
    );
    assert!(counters_from(&values, &[]).is_empty());
}

#[test]
fn the_json_has_the_kind_and_only_that_kinds_fields() {
    let json = serde_json::to_value([
        Counter::of_stats(CounterKind::BloodGem, 4, 3),
        Counter::of_attack(CounterKind::UndeadAttack, 7),
        Counter::of_count(CounterKind::EternalKnights, 5),
        Counter::of_stats(CounterKind::ShopElemental, 4, 4),
    ])
    .expect("serializable");
    assert_eq!(
        json,
        serde_json::json!([
            { "kind": "blood_gem", "atk": 4, "health": 3 },
            { "kind": "undead_attack", "atk": 7 },
            { "kind": "eternal_knights", "count": 5 },
            { "kind": "shop_elemental", "atk": 4, "health": 4 },
        ])
    );
}

#[test]
fn the_values_the_simulator_gets_are_the_same_as_before() {
    // The counters read the same tags as the simulator's values: those stay
    // the printed numbers, with no total and no turn gate change.
    let parts = vec![
        vec![
            tag("2", "BACON_BLOODGEMBUFFATKVALUE", "3"),
            tag("2", "BACON_BLOODGEMBUFFHEALTHVALUE", "2"),
        ],
        enchantment(60, "BG25_011pe", 2, "PLAY", (7, 0)),
    ];
    let reader = reader_with(&parts);
    let values = reader.own_values_now().expect("a shop turn");
    assert_eq!(values.blood_gem, (3, 2), "the extra, not the total");
    assert_eq!(values.undead_attack, 7);
    assert_eq!(values.spell, (0, 0));
    let before = serde_json::to_string(&reader.snapshot_current().expect("a game")).unwrap();
    let _ = reader.counters_now();
    let after = serde_json::to_string(&reader.snapshot_current().expect("a game")).unwrap();
    assert_eq!(before, after);

    let mut combat = parts;
    combat.push(vec![tag("GameEntity", "TURN", "2")]);
    assert_eq!(
        reader_with(&combat).own_values_now(),
        None,
        "the shop-turn read keeps its turn gate"
    );
}
