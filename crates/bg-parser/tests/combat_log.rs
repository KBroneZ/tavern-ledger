//! What the combat simulator reads from the parser (T-603b, T-604, T-605):
//! each combat's attacks in order with both boards before each one, the
//! lasting keywords the saved report leaves out, the own board in a shop
//! turn and each side's game-long values. Synthetic log written out below;
//! of all this, only the game-long values (parser revision 10) and the
//! buffs on board minions (revision 14) reach the saved report's JSON.

use bg_parser::report::{CombatValues, DefenderSource, Deity, GameReport, Side, MAX_GAME_VALUE};
use bg_parser::LogReader;

const T: &str = "D 12:00:00.0000000 GameState.DebugPrintPower() - ";

fn line(indent: usize, body: &str) -> String {
    format!("{T}{}{body}", "    ".repeat(indent))
}

fn minion(
    id: i64,
    card: &str,
    controller: i64,
    pos: i64,
    atk: i64,
    hp: i64,
    tags: &[&str],
) -> Vec<String> {
    let mut out = vec![line(
        1,
        &format!("FULL_ENTITY - Creating ID={id} CardID={card}"),
    )];
    for (tag, value) in [
        ("CARDTYPE", "MINION".to_string()),
        ("CONTROLLER", controller.to_string()),
        ("ZONE", "PLAY".to_string()),
        ("ATK", atk.to_string()),
        ("HEALTH", hp.to_string()),
        ("ZONE_POSITION", pos.to_string()),
    ] {
        out.push(line(2, &format!("tag={tag} value={value}")));
    }
    for tag in tags {
        out.push(line(2, &format!("tag={tag} value=1")));
    }
    out.push(line(2, &format!("tag=ENTITY_ID value={id}")));
    out
}

fn hero(id: i64, card: &str, controller: i64, pid: i64, place: i64, zone: &str) -> Vec<String> {
    let mut out = vec![line(
        1,
        &format!("FULL_ENTITY - Creating ID={id} CardID={card}"),
    )];
    out.push(line(2, &format!("tag=CONTROLLER value={controller}")));
    out.push(line(2, &format!("tag=ZONE value={zone}")));
    if pid > 0 {
        out.push(line(2, &format!("tag=PLAYER_ID value={pid}")));
        out.push(line(
            2,
            &format!("tag=PLAYER_LEADERBOARD_PLACE value={place}"),
        ));
    }
    out.push(line(2, "tag=CARDTYPE value=HERO"));
    out.push(line(2, "tag=HEALTH value=30"));
    out.push(line(2, &format!("tag=ENTITY_ID value={id}")));
    out
}

fn tag(entity: &str, tag: &str, value: &str) -> String {
    line(
        1,
        &format!("TAG_CHANGE Entity={entity} tag={tag} value={value} "),
    )
}

fn attack(attacker: i64) -> String {
    line(
        0,
        &format!(
            "BLOCK_START BlockType=ATTACK Entity={attacker} EffectCardId=System.Collections.Generic.List`1[System.String] EffectIndex=0 Target=0 SubOption=-1 "
        ),
    )
}

/// A Solo game: shop turn 1 with two own minions (one Poisonous, one
/// Mega-Windfury), then combat 1 with two attacks: the first names its
/// defender (`PROPOSED_DEFENDER`), the second only damages it.
fn game() -> Vec<String> {
    let mut l = vec![
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
    ];
    l.extend(hero(102, "SYN_HERO_A", 2, 2, 1, "PLAY"));
    l.extend(hero(21, "SYN_HERO_B", 10, 3, 2, "SETASIDE"));
    l.push(tag("2", "HERO_ENTITY", "102"));
    l.push(tag("GameEntity", "TURN", "1"));
    l.extend(minion(20, "SYN_POISON", 2, 1, 1, 1, &["POISONOUS"]));
    l.extend(minion(
        21,
        "SYN_MEGA",
        2,
        2,
        3,
        3,
        &["MEGA_WINDFURY", "TAUNT"],
    ));
    // Any later line closes the last minion's packet.
    l.push(tag("2", "RESOURCES", "1"));
    l.push(tag("GameEntity", "TURN", "2"));
    // The combat plays copies; the shop's minions wait set aside.
    l.push(tag("20", "ZONE", "SETASIDE"));
    l.push(tag("21", "ZONE", "SETASIDE"));
    l.extend(hero(44, "SYN_HERO_B", 10, 0, 0, "PLAY"));
    l.push(tag("3", "HERO_ENTITY", "44"));
    l.extend(minion(30, "SYN_POISON", 2, 1, 1, 1, &["POISONOUS"]));
    l.extend(minion(
        31,
        "SYN_MEGA",
        2,
        2,
        3,
        3,
        &["MEGA_WINDFURY", "TAUNT"],
    ));
    l.extend(minion(40, "SYN_FOE_A", 10, 1, 2, 4, &["DIVINE_SHIELD"]));
    l.extend(minion(41, "SYN_FOE_B", 10, 2, 1, 2, &[]));
    l.push(tag("GameEntity", "NEXT_STEP", "MAIN_ACTION"));
    l.push(attack(30));
    l.push(tag("GameEntity", "PROPOSED_ATTACKER", "30"));
    l.push(tag("GameEntity", "PROPOSED_DEFENDER", "41"));
    l.push(tag("40", "DAMAGE", "1"));
    l.push(tag("41", "DAMAGE", "1"));
    l.push(line(0, "BLOCK_END"));
    l.push(attack(40));
    l.push(tag("30", "DAMAGE", "2"));
    l.push(tag("40", "DAMAGE", "2"));
    l.push(line(0, "BLOCK_END"));
    l.push(tag("GameEntity", "TURN", "3"));
    l.push(tag("GameEntity", "STATE", "COMPLETE"));
    l
}

fn report(lines: &[String]) -> GameReport {
    let mut reader = LogReader::default();
    lines.iter().for_each(|l| reader.feed(l));
    let mut reports = reader.finish();
    assert_eq!(reports.len(), 1);
    let r = reports.remove(0);
    assert!(r.problems.is_empty(), "{:?} {:?}", r.status, r.problems);
    r
}

#[test]
fn each_attack_keeps_its_attacker_defender_and_both_boards() {
    let r = report(&game());
    let attacks = &r.rounds[0].attacks;
    assert_eq!(attacks.len(), 2);
    let first = &attacks[0];
    assert_eq!(first.attacker.side, Side::Own);
    assert_eq!(first.attacker.position, 1);
    assert_eq!(first.attacker.card_id.as_deref(), Some("SYN_POISON"));
    let (defender, source) = first.defender.clone().expect("named by the game");
    // The game's word wins over the first minion damaged (40).
    assert_eq!((defender.side, defender.position), (Side::Opponent, 2));
    assert_eq!(source, DefenderSource::Game);
    assert_eq!((first.own.len(), first.opponent.len()), (2, 2));
    assert_eq!(first.opponent[0].health, 4);

    let second = &attacks[1];
    assert_eq!(second.attacker.side, Side::Opponent);
    let (defender, source) = second.defender.clone().expect("the first enemy hit");
    assert_eq!(defender.card_id.as_deref(), Some("SYN_POISON"));
    assert_eq!(source, DefenderSource::FirstHit);
    // The board before the second attack shows the first one's damage.
    assert_eq!(second.opponent[0].health, 3);
}

#[test]
fn poisonous_and_mega_windfury_are_read_but_not_saved() {
    let r = report(&game());
    let own = r.rounds[0].entries[0].board.clone().expect("own board");
    assert_eq!(own[0].sim_keywords, ["poisonous"]);
    assert_eq!(own[1].sim_keywords, ["mega_windfury"]);
    assert_eq!(own[1].keywords, ["taunt"]);
    let json = serde_json::to_string(&r).expect("serializable");
    assert!(!json.contains("poisonous") && !json.contains("mega_windfury"));
    assert!(!json.contains("attacks") && !json.contains("PROPOSED"));
}

#[test]
fn a_mark_given_at_the_start_of_combat_is_on_the_first_attack_board_only() {
    // Fragrant Phylactery fires before the first attack and marks the own
    // Poisonous minion (T-631).
    let mut all = game();
    let first_attack = all
        .iter()
        .position(|l| l.contains("BlockType=ATTACK"))
        .unwrap();
    let mut power = vec![line(
        1,
        "FULL_ENTITY - Creating ID=50 CardID=BG20_HERO_282p",
    )];
    for (t, v) in [
        ("CARDTYPE", "HERO_POWER"),
        ("CONTROLLER", "2"),
        ("ZONE", "PLAY"),
    ] {
        power.push(line(2, &format!("tag={t} value={v}")));
    }
    power.push(line(2, "tag=ENTITY_ID value=50"));
    power.push(line(
        0,
        "BLOCK_START BlockType=TRIGGER Entity=50 EffectCardId=System.Collections.Generic.List`1[System.String] EffectIndex=4 Target=0 SubOption=-1 TriggerKeyword=TRIGGER_VISUAL",
    ));
    power.push(line(
        1,
        "FULL_ENTITY - Creating ID=60 CardID=BG20_HERO_282e2",
    ));
    for (t, v) in [
        ("CARDTYPE", "ENCHANTMENT"),
        ("CONTROLLER", "2"),
        ("ZONE", "PLAY"),
        ("ATTACHED", "30"),
    ] {
        power.push(line(2, &format!("tag={t} value={v}")));
    }
    power.push(line(2, "tag=ENTITY_ID value=60"));
    power.push(line(0, "BLOCK_END"));
    all.splice(first_attack..first_attack, power);
    let r = report(&all);
    let entry = &r.rounds[0].entries[0];
    let at_attack = entry.board.clone().expect("own board");
    assert_eq!(at_attack[0].sim_marks, ["BG20_HERO_282e2"]);
    assert!(at_attack[1].sim_marks.is_empty());
    let before = entry
        .before
        .clone()
        .expect("the board before start of combat");
    assert!(before.iter().all(|m| m.sim_marks.is_empty()), "{before:?}");
    let json = serde_json::to_string(&r).expect("serializable");
    assert!(!json.contains("282e2") && !json.contains("sim_marks"));
}

#[test]
fn the_own_board_now_is_given_in_a_shop_turn_only() {
    let all = game();
    let at = |needle: &str| all.iter().position(|l| l.contains(needle)).unwrap();
    let mut reader = LogReader::default();
    all[..at("tag=TURN value=2")]
        .iter()
        .for_each(|l| reader.feed(l));
    let board = reader.own_board_now().expect("shop turn 1");
    let cards: Vec<_> = board.iter().map(|m| m.card_id.as_deref()).collect();
    assert_eq!(cards, [Some("SYN_POISON"), Some("SYN_MEGA")]);
    assert_eq!(board[1].sim_keywords, ["mega_windfury"]);

    all[at("tag=TURN value=2")..at("tag=TURN value=3")]
        .iter()
        .for_each(|l| reader.feed(l));
    assert!(reader.own_board_now().is_none(), "a combat turn");
    all[at("tag=TURN value=3")..]
        .iter()
        .for_each(|l| reader.feed(l));
    assert!(reader.own_board_now().is_none(), "the game is over");
    assert!(LogReader::default().own_board_now().is_none());
}

#[test]
fn each_round_keeps_the_own_shop_board_before_it_but_does_not_save_it() {
    let r = report(&game());
    let board = r.rounds[0]
        .own_shop_board
        .clone()
        .expect("shop turn 1 seen");
    let cards: Vec<_> = board.iter().map(|m| m.card_id.as_deref()).collect();
    assert_eq!(cards, [Some("SYN_POISON"), Some("SYN_MEGA")]);
    assert_eq!(board[0].sim_keywords, ["poisonous"]);
    let json = serde_json::to_string(&r).expect("serializable");
    assert!(!json.contains("shop_board"));

    // A log that starts in the combat has no shop turn before it.
    let all = game();
    let from = all
        .iter()
        .position(|l| l.contains("tag=TURN value=1"))
        .unwrap();
    let mut late: Vec<String> = all[..from].to_vec();
    late.extend(all[from + 1..].iter().cloned());
    let r = report(&late);
    assert_eq!(r.rounds[0].own_shop_board, None);
}

/// The game-long value tags and player enchantments (T-609), as the log
/// gives them, for the player (entity 2) and the opponent (the dummy
/// player, entity 3): the player's Blood Gems and Undead Attack, the
/// opponent's Tavern spells, Eternal Knights and Beetles; the player's set
/// aside Knights are not in play.
fn value_lines() -> Vec<String> {
    let mut values = vec![
        tag("2", "BACON_BLOODGEMBUFFATKVALUE", "3"),
        tag("2", "BACON_BLOODGEMBUFFHEALTHVALUE", "2"),
        tag("3", "TAVERN_SPELL_ATTACK_INCREASE", "1"),
        tag("3", "TAVERN_SPELL_HEALTH_INCREASE", "4"),
    ];
    for (id, card, player, zone, nums) in [
        (60, "BG25_011pe", 2, "PLAY", (7, 0)),
        (61, "BG31_808pe", 3, "PLAY", (12, 11)),
        (62, "BG25_008pe", 2, "SETASIDE", (9, 0)),
        (63, "BG25_008pe", 3, "PLAY", (2, 0)),
    ] {
        values.push(line(
            1,
            &format!("FULL_ENTITY - Creating ID={id} CardID={card}"),
        ));
        values.push(line(2, "tag=CARDTYPE value=ENCHANTMENT"));
        values.push(line(2, &format!("tag=ATTACHED value={player}")));
        values.push(line(2, &format!("tag=ZONE value={zone}")));
        values.push(line(
            2,
            &format!("tag=TAG_SCRIPT_DATA_NUM_1 value={}", nums.0),
        ));
        values.push(line(
            2,
            &format!("tag=TAG_SCRIPT_DATA_NUM_2 value={}", nums.1),
        ));
        values.push(line(2, &format!("tag=ENTITY_ID value={id}")));
    }
    // The player's hidden Deities (T-671): an older one still set aside and
    // this combat's, the newest; the opponent's is not in the log.
    for (id, (atk, health, progress)) in [(64, (1, 1, 0)), (65, (77, 64, 1))] {
        values.push(line(
            1,
            &format!("FULL_ENTITY - Creating ID={id} CardID=BG_OldGod"),
        ));
        for (name, value) in [
            ("CONTROLLER", 2),
            ("TAG_SCRIPT_DATA_NUM_1", 4),
            ("TAG_SCRIPT_DATA_NUM_2", atk),
            ("TAG_SCRIPT_DATA_NUM_3", health),
            ("QUEST_PROGRESS", progress),
            ("BACON_EVOLUTION_CARD_ID", 130_610),
        ] {
            values.push(line(2, &format!("tag={name} value={value}")));
        }
        values.push(line(2, "tag=CARDTYPE value=SPELL"));
        values.push(line(2, "tag=ZONE value=SETASIDE"));
        values.push(line(2, &format!("tag=ENTITY_ID value={id}")));
    }
    values.push(tag("2", "RESOURCES", "1"));
    values
}

/// [`game`] with [`value_lines`] just before the line that holds `needle`.
fn game_with_values(needle: &str) -> Vec<String> {
    let mut l = game();
    let at = l.iter().position(|x| x.contains(needle)).expect("needle");
    l.splice(at..at, value_lines());
    l
}

#[test]
fn each_sides_game_long_values_are_read_with_its_board() {
    // T-609: the player's and the opponent's (the dummy player's) values as
    // the boards are taken at the first attack.
    let r = report(&game_with_values("BlockType=ATTACK"));
    let entries = &r.rounds[0].entries;
    let of = |side: Side| {
        entries
            .iter()
            .find(|e| e.side == side)
            .and_then(|e| e.values)
            .expect("values with the board")
    };
    assert_eq!(of(Side::Own).blood_gem, (3, 2));
    assert_eq!(of(Side::Own).spell, (0, 0));
    assert_eq!(of(Side::Opponent).spell, (1, 4));
    assert_eq!(of(Side::Opponent).blood_gem, (0, 0));
    assert_eq!(of(Side::Own).undead_attack, 7);
    assert_eq!(of(Side::Own).knights_dead, 0, "set aside: not read");
    assert_eq!(of(Side::Opponent).beetle, (12, 11));
    assert_eq!(of(Side::Opponent).undead_attack, 0);
    assert_eq!(of(Side::Opponent).knights_dead, 2);
    // The newest Deity set aside under the player; none for the opponent.
    assert_eq!(
        of(Side::Own).deity,
        Deity {
            dbf: 130_610,
            attack: 77,
            health: 64,
            needed: 4,
            progress: 1,
        }
    );
    assert_eq!(of(Side::Opponent).deity, Deity::default());
}

/// Enchantments on the combat's minions (T-619): the own 30 has two Blood
/// Gems, the game's hidden holder of their numbers and a Dark Gift; the own
/// 31 a buff that left; the opponent's 40 one buff in two forms.
fn buff_lines() -> Vec<String> {
    let mut lines = Vec::new();
    for (id, card, on, zone, extra) in [
        (70, "BG20_GEMe2", 30, "PLAY", None),
        (71, "BG20_GEMe2", 30, "PLAY", None),
        (72, "BG20_GEMe", 30, "PLAY", Some("ENCHANTMENT_INVISIBLE")),
        (73, "BG36_MidGameEffect_000t9e2", 30, "PLAY", None),
        (74, "BG28_309e", 31, "GRAVEYARD", None),
        (75, "BG34_950e", 40, "PLAY", None),
        (76, "BG34_950e2", 40, "PLAY", None),
    ] {
        lines.push(line(
            1,
            &format!("FULL_ENTITY - Creating ID={id} CardID={card}"),
        ));
        lines.push(line(2, "tag=CARDTYPE value=ENCHANTMENT"));
        lines.push(line(2, &format!("tag=ATTACHED value={on}")));
        lines.push(line(2, &format!("tag=ZONE value={zone}")));
        if let Some(tag) = extra {
            lines.push(line(2, &format!("tag={tag} value=1")));
        }
        lines.push(line(2, &format!("tag=ENTITY_ID value={id}")));
    }
    lines.push(tag("2", "RESOURCES", "1"));
    lines
}

fn game_with_buffs() -> Vec<String> {
    let mut l = game();
    let at = l
        .iter()
        .position(|x| x.contains("BlockType=ATTACK"))
        .expect("attack");
    l.splice(at..at, buff_lines());
    l
}

#[test]
fn buffs_are_read_with_each_sides_minions() {
    let r = report(&game_with_buffs());
    let board = |side: Side| {
        r.rounds[0]
            .entries
            .iter()
            .find(|e| e.side == side)
            .and_then(|e| e.board.clone())
            .expect("board")
    };
    let own = board(Side::Own);
    fn kinds(m: &bg_parser::report::Minion) -> Vec<(&str, u32)> {
        m.buffs
            .iter()
            .map(|b| (b.card_id.as_str(), b.count))
            .collect()
    }
    assert_eq!(kinds(&own[0]), vec![("BG20_GEM", 2)]);
    assert!(own[1].buffs.is_empty(), "a buff that left is not one");
    let foe = board(Side::Opponent);
    assert_eq!(kinds(&foe[0]), vec![("BG34_950", 2)]);
    assert!(foe[1].buffs.is_empty());
}

#[test]
fn buffs_are_saved_with_the_minion_and_left_out_when_there_are_none() {
    let json = serde_json::to_value(report(&game_with_buffs())).expect("serializable");
    let boards: Vec<&serde_json::Value> = json["rounds"][0]["entries"]
        .as_array()
        .expect("entries")
        .iter()
        .flat_map(|e| e["board"].as_array().expect("board"))
        .collect();
    let with: Vec<_> = boards.iter().filter(|m| m.get("buffs").is_some()).collect();
    assert_eq!(with.len(), 2);
    assert_eq!(
        with[0]["buffs"],
        serde_json::json!([{"card_id": "BG20_GEM", "count": 2}])
    );
    let plain = serde_json::to_string(&report(&game())).expect("serializable");
    assert!(!plain.contains("buffs"), "{plain}");
}

#[test]
fn each_sides_game_long_values_are_saved_with_its_board_above_zero_only() {
    // Parser revision 10: the values above zero go with each board in the
    // saved report; zero is left out, so an older record (no key) and a
    // value the log never raised read the same: the printed numbers.
    let r = report(&game_with_values("BlockType=ATTACK"));
    let json = serde_json::to_value(&r).expect("serializable");
    let entries = json["rounds"][0]["entries"].as_array().expect("entries");
    let of = |side: &str| {
        entries
            .iter()
            .find(|e| e["side"] == side)
            .map(|e| e["values"].clone())
            .expect("entry")
    };
    assert_eq!(
        of("own"),
        serde_json::json!({"blood_gem_atk": 3, "blood_gem_health": 2, "undead_atk": 7,
                           "deity_dbf": 130_610, "deity_atk": 77, "deity_health": 64,
                           "deity_needed": 4, "deity_progress": 1})
    );
    assert_eq!(
        of("opponent"),
        serde_json::json!({
            "spell_atk": 1, "spell_health": 4, "knights_dead": 2,
            "beetle_atk": 12, "beetle_health": 11
        })
    );
}

#[test]
fn a_board_with_no_game_long_value_above_zero_saves_no_values() {
    let r = report(&game());
    assert!(r.rounds[0].entries.iter().all(|e| e.values.is_some()));
    let json = serde_json::to_string(&r).expect("serializable");
    assert!(!json.contains("\"values\""), "{json}");
}

#[test]
fn game_long_values_outside_the_saved_bounds_are_left_out() {
    // Negative or past MAX_GAME_VALUE: not a number the game shows; left
    // out (unknown) rather than refused by upload-game.
    let values = CombatValues {
        blood_gem: (-1, MAX_GAME_VALUE + 1),
        spell: (MAX_GAME_VALUE, 1),
        knights_dead: 0,
        undead_attack: i64::MAX,
        beetle: (i64::MIN, 2),
        deity: Deity {
            dbf: 130_610,
            attack: 77,
            health: MAX_GAME_VALUE + 1,
            needed: 4,
            progress: 0,
        },
    };
    assert_eq!(
        serde_json::to_value(values).expect("serializable"),
        serde_json::json!({"spell_atk": MAX_GAME_VALUE, "spell_health": 1, "beetle_health": 2,
                           "deity_dbf": 130_610, "deity_atk": 77, "deity_needed": 4})
    );
    assert!(!values.is_empty());
    assert!(CombatValues {
        blood_gem: (-3, 0),
        ..CombatValues::default()
    }
    .is_empty());
}

#[test]
fn the_own_game_long_values_now_are_given_in_a_shop_turn_only() {
    // T-610: the overlay's odds play the player's values as they are now.
    let all = game_with_values("tag=TURN value=2");
    let at = |needle: &str| all.iter().position(|l| l.contains(needle)).unwrap();
    let mut reader = LogReader::default();
    assert_eq!(reader.own_values_now(), None);
    all[..at("tag=TURN value=2")]
        .iter()
        .for_each(|l| reader.feed(l));
    let now = reader.own_values_now().expect("shop turn 1");
    assert_eq!(now.blood_gem, (3, 2));
    assert_eq!(now.undead_attack, 7);
    assert_eq!(now.spell, (0, 0), "the opponent's, not the player's");
    all[at("tag=TURN value=2")..at("tag=TURN value=3")]
        .iter()
        .for_each(|l| reader.feed(l));
    assert_eq!(reader.own_values_now(), None, "a combat turn");
}

/// The parser's report of the game with buffs (parser revision 14), saved for
/// upload-game's Deno tests and the key contract.
/// `BLESS=1 cargo test -p bg-parser --test combat_log` writes it again.
#[test]
fn the_saved_report_with_buffs_for_upload_game_is_current() {
    let reports = vec![report(&game_with_buffs())];
    let text = serde_json::to_string_pretty(&reports).unwrap() + "\n";
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data/rust/game_buffs_games.json");
    if std::env::var_os("BLESS").is_some() {
        std::fs::write(&path, &text).unwrap();
    }
    let saved = std::fs::read_to_string(&path)
        .unwrap_or_default()
        .replace("\r\n", "\n");
    assert_eq!(
        saved, text,
        "run BLESS=1 cargo test -p bg-parser --test combat_log"
    );
}

/// The parser's report of the game with values, saved for upload-game's Deno
/// tests and the key contract (`tests/upload_contract.rs`).
/// `BLESS=1 cargo test -p bg-parser --test combat_log` writes it again.
#[test]
fn the_saved_report_with_values_for_upload_game_is_current() {
    let reports = vec![report(&game_with_values("BlockType=ATTACK"))];
    let text = serde_json::to_string_pretty(&reports).unwrap() + "\n";
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data/rust/game_values_games.json");
    if std::env::var_os("BLESS").is_some() {
        std::fs::write(&path, &text).unwrap();
    }
    let saved = std::fs::read_to_string(&path)
        .unwrap_or_default()
        .replace("\r\n", "\n");
    assert_eq!(
        saved, text,
        "run BLESS=1 cargo test -p bg-parser --test combat_log"
    );
}

#[test]
fn the_combat_round_now_is_given_in_a_combat_turn_only() {
    let all = game();
    let at = |needle: &str| all.iter().position(|l| l.contains(needle)).unwrap();
    let mut reader = LogReader::default();
    assert_eq!(reader.combat_round_now(), None, "no game");
    all[..at("tag=TURN value=2")]
        .iter()
        .for_each(|l| reader.feed(l));
    assert_eq!(reader.combat_round_now(), None, "shop turn 1");
    all[at("tag=TURN value=2")..at("tag=TURN value=3")]
        .iter()
        .for_each(|l| reader.feed(l));
    assert_eq!(reader.combat_round_now(), Some(1), "the first combat");
    all[at("tag=TURN value=3")..]
        .iter()
        .for_each(|l| reader.feed(l));
    assert_eq!(reader.combat_round_now(), None, "the game is over");
}

/// The boards that have entered play in the combat on now, line by line, in a
/// real game: `(combat round, boards in)` each time it changes.
fn boards_in_over(fixture: &str) -> Vec<(i64, usize)> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(format!("tests/data/real/{fixture}.log"));
    let text = std::fs::read_to_string(path)
        .expect("fixture")
        .replace("\r\n", "\n");
    let mut reader = LogReader::default();
    let mut seen: Vec<(i64, usize)> = Vec::new();
    for l in text.lines() {
        reader.feed(l);
        let Some(round) = reader.combat_round_now() else {
            continue;
        };
        if let Some(boards) = reader.combat_boards_in_now() {
            if seen.last() != Some(&(round, boards)) {
                seen.push((round, boards));
            }
        }
    }
    seen
}

#[test]
fn a_solo_combat_has_both_boards_in_at_its_first_attack() {
    let seen = boards_in_over("b253216_solo");
    assert!(seen.len() >= 3, "several combats: {seen:?}");
    assert!(
        seen.iter().all(|&(_, boards)| boards == 2),
        "own and opponent board enter together: {seen:?}"
    );
    let rounds: Vec<i64> = seen.iter().map(|s| s.0).collect();
    assert!(rounds.windows(2).all(|w| w[0] < w[1]), "once per combat");
}

#[test]
fn a_duos_combat_gains_a_board_each_time_one_enters_play() {
    let seen = boards_in_over("b253216_duos");
    for round in seen.iter().map(|s| s.0) {
        let counts: Vec<usize> = seen.iter().filter(|s| s.0 == round).map(|s| s.1).collect();
        assert!(
            counts.windows(2).all(|w| w[0] < w[1]),
            "round {round}: never fewer boards than before: {counts:?}"
        );
    }
    assert!(
        seen.iter().any(|&(_, boards)| boards == 4),
        "all four boards of a team fight enter play in some combat: {seen:?}"
    );
}
