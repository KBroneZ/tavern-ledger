//! What the parser keeps for the simulator's start of combat (T-628): each
//! combat leg's boards just before the first start-of-combat effect and each
//! side's hero powers and trinkets then. Rust-only: none of it reaches the
//! saved report's JSON. Synthetic log written out below.

use bg_parser::report::{GameReport, Side};
use bg_parser::LogReader;

const T: &str = "D 12:00:00.0000000 GameState.DebugPrintPower() - ";

fn line(indent: usize, body: &str) -> String {
    format!("{T}{}{body}", "    ".repeat(indent))
}

fn entity(indent: usize, id: i64, card: &str, tags: &[(&str, String)]) -> Vec<String> {
    let mut out = vec![line(
        indent,
        &format!("FULL_ENTITY - Creating ID={id} CardID={card}"),
    )];
    for (tag, value) in tags {
        out.push(line(indent + 1, &format!("tag={tag} value={value}")));
    }
    out.push(line(indent + 1, &format!("tag=ENTITY_ID value={id}")));
    out
}

fn minion(id: i64, card: &str, controller: i64, pos: i64, atk: i64, hp: i64) -> Vec<String> {
    entity(
        1,
        id,
        card,
        &[
            ("CARDTYPE", "MINION".into()),
            ("CONTROLLER", controller.to_string()),
            ("ZONE", "PLAY".into()),
            ("ATK", atk.to_string()),
            ("HEALTH", hp.to_string()),
            ("ZONE_POSITION", pos.to_string()),
        ],
    )
}

fn hero(id: i64, card: &str, controller: i64, pid: i64, zone: &str) -> Vec<String> {
    let mut tags = vec![
        ("CONTROLLER", controller.to_string()),
        ("ZONE", zone.to_string()),
        ("CARDTYPE", "HERO".to_string()),
        ("HEALTH", "30".to_string()),
    ];
    if pid > 0 {
        tags.push(("PLAYER_ID", pid.to_string()));
        tags.push(("PLAYER_LEADERBOARD_PLACE", pid.to_string()));
    }
    entity(1, id, card, &tags)
}

/// A hero power or trinket in play for `controller`, with its script numbers.
fn kit_card(id: i64, card: &str, kind: &str, controller: i64, nums: &[(&str, i64)]) -> Vec<String> {
    let mut tags = vec![
        ("CARDTYPE", kind.to_string()),
        ("CONTROLLER", controller.to_string()),
        ("ZONE", "PLAY".to_string()),
    ];
    for (tag, value) in nums {
        tags.push((tag, value.to_string()));
    }
    entity(1, id, card, &tags)
}

fn tag(entity: &str, tag: &str, value: &str) -> String {
    line(
        1,
        &format!("TAG_CHANGE Entity={entity} tag={tag} value={value} "),
    )
}

fn block(kind: &str, entity: &str, indent: usize) -> String {
    line(
        indent,
        &format!(
            "BLOCK_START BlockType={kind} Entity={entity} EffectCardId=System.Collections.Generic.List`1[System.String] EffectIndex=0 Target=0 SubOption=-1 TriggerKeyword=TAG_NOT_SET"
        ),
    )
}

fn end(indent: usize) -> String {
    line(indent, "BLOCK_END")
}

fn header(game_type: &str) -> Vec<String> {
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
        "D 12:00:00.0000000 GameState.DebugPrintGame() - BuildNumber=254477".into(),
        format!("D 12:00:00.0000000 GameState.DebugPrintGame() - GameType={game_type}"),
    ]
}

/// A Solo game: the player's hero power (with a counter and a chosen
/// minion) and two trinket slots (a lesser trinket bought, the greater one
/// still a placeholder) in the shop; combat 1, where the game's own setup
/// block sets the opponent up (hero, hero power, one trinket, two minions),
/// a cosmetic pet fires, then the opponent's minion gives itself +2/+2 at
/// the start of combat, then the first attack.
fn solo() -> Vec<String> {
    let mut l = header("GT_BATTLEGROUNDS");
    l.extend(hero(102, "SYN_HERO_A", 2, 2, "PLAY"));
    l.extend(hero(21, "SYN_HERO_B", 10, 3, "SETASIDE"));
    l.push(tag("2", "HERO_ENTITY", "102"));
    l.push(tag("GameEntity", "TURN", "1"));
    l.extend(minion(20, "SYN_OWN_A", 2, 1, 2, 3));
    l.extend(minion(22, "SYN_OWN_B", 2, 2, 1, 1));
    l.extend(kit_card(
        50,
        "SYN_POWER",
        "HERO_POWER",
        2,
        &[("TAG_SCRIPT_DATA_NUM_1", 5), ("TAG_SCRIPT_DATA_ENT_1", 22)],
    ));
    l.extend(kit_card(
        51,
        "SYN_LESSER",
        "BATTLEGROUND_TRINKET",
        2,
        &[("TAG_SCRIPT_DATA_NUM_6", 1), ("TAG_SCRIPT_DATA_NUM_2", 3)],
    ));
    l.extend(kit_card(
        52,
        "BG30_Trinket_2nd",
        "BATTLEGROUND_TRINKET",
        2,
        &[("TAG_SCRIPT_DATA_NUM_6", 2), ("TAG_SCRIPT_DATA_NUM_1", 4)],
    ));
    l.push(tag("2", "RESOURCES", "1"));
    l.push(tag("GameEntity", "TURN", "2"));
    // The game's setup: a top-level block of its own rules entity.
    l.extend(entity(
        0,
        60,
        "TB_BaconShop_8P_PlayerE",
        &[
            ("CARDTYPE", "ENCHANTMENT".into()),
            ("CONTROLLER", "2".into()),
        ],
    ));
    l.push(block("TRIGGER", "60", 0));
    l.extend(hero(44, "SYN_HERO_B", 10, 0, "PLAY"));
    l.push(tag("3", "HERO_ENTITY", "44"));
    l.extend(kit_card(
        45,
        "SYN_FOE_POWER",
        "HERO_POWER",
        10,
        &[("TAG_SCRIPT_DATA_NUM_1", 2)],
    ));
    l.extend(kit_card(
        46,
        "SYN_GREATER",
        "BATTLEGROUND_TRINKET",
        10,
        &[("TAG_SCRIPT_DATA_NUM_6", 2)],
    ));
    l.extend(minion(40, "SYN_FOE_A", 10, 1, 4, 4));
    l.extend(minion(41, "SYN_FOE_B", 10, 2, 1, 2));
    l.push(end(0));
    // A cosmetic pet's trigger is not the start of combat.
    l.extend(entity(
        0,
        61,
        "PET_1_1",
        &[("CARDTYPE", "PET".into()), ("CONTROLLER", "10".into())],
    ));
    l.push(block("TRIGGER", "61", 0));
    l.push(end(0));
    l.push(tag("GameEntity", "STEP", "MAIN_START"));
    // The opponent's start of combat.
    l.push(block("TRIGGER", "40", 0));
    l.push(tag("40", "ATK", "6"));
    l.push(tag("40", "HEALTH", "6"));
    l.push(end(0));
    l.push(block("ATTACK", "20", 0));
    l.push(tag("GameEntity", "PROPOSED_DEFENDER", "41"));
    l.push(tag("41", "DAMAGE", "2"));
    l.push(tag("20", "DAMAGE", "1"));
    l.push(end(0));
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

fn stats(board: &[bg_parser::report::Minion]) -> Vec<(&str, i64, i64)> {
    board
        .iter()
        .map(|m| (m.card_id.as_deref().unwrap_or("?"), m.atk, m.health))
        .collect()
}

#[test]
fn each_board_is_kept_as_it_was_before_the_start_of_combat() {
    let r = report(&solo());
    let entries = &r.rounds[0].entries;
    let of = |side: Side| entries.iter().find(|e| e.side == side).expect("entry");
    let opp = of(Side::Opponent);
    // Today's board, at the first attack, carries the +2/+2.
    assert_eq!(
        stats(opp.board.as_deref().unwrap()),
        [("SYN_FOE_A", 6, 6), ("SYN_FOE_B", 1, 2)]
    );
    // The board before it does not.
    assert_eq!(
        stats(opp.before.as_deref().expect("before start of combat")),
        [("SYN_FOE_A", 4, 4), ("SYN_FOE_B", 1, 2)]
    );
    let own = of(Side::Own);
    assert_eq!(
        stats(own.before.as_deref().expect("before start of combat")),
        [("SYN_OWN_A", 2, 3), ("SYN_OWN_B", 1, 1)]
    );
}

#[test]
fn each_side_keeps_its_hero_powers_and_trinkets_with_their_numbers() {
    let r = report(&solo());
    let entries = &r.rounds[0].entries;
    let kit = |side: Side| {
        entries
            .iter()
            .find(|e| e.side == side)
            .and_then(|e| e.kit.clone())
            .expect("kit")
    };
    let own = kit(Side::Own);
    assert_eq!(own.powers.len(), 1);
    assert_eq!(own.powers[0].card_id, "SYN_POWER");
    assert_eq!(own.powers[0].slot, 0);
    assert_eq!(own.powers[0].counters[0], 5);
    // The minion it names, by its board position.
    assert_eq!(own.powers[0].target, Some(2));
    let trinkets: Vec<_> = own
        .trinkets
        .iter()
        .map(|t| (t.card_id.as_str(), t.slot, t.counters))
        .collect();
    assert_eq!(
        trinkets,
        [
            ("SYN_LESSER", 1, [0, 3, 0, 0, 0]),
            ("BG30_Trinket_2nd", 2, [4, 0, 0, 0, 0])
        ]
    );
    let opp = kit(Side::Opponent);
    assert_eq!(opp.powers[0].card_id, "SYN_FOE_POWER");
    assert_eq!(opp.powers[0].counters[0], 2);
    assert_eq!(opp.powers[0].target, None);
    assert_eq!(opp.trinkets.len(), 1);
    assert_eq!(
        (opp.trinkets[0].card_id.as_str(), opp.trinkets[0].slot),
        ("SYN_GREATER", 2)
    );
}

/// `solo()` with the player's hero power naming entity `named` instead of
/// its board minion, and `extra` lines added before the combat turn.
fn solo_naming(named: i64, extra: Vec<String>) -> Vec<String> {
    let mut l = solo();
    let at = l
        .iter()
        .position(|x| x.contains("tag=TAG_SCRIPT_DATA_ENT_1 value=22"))
        .expect("the power names a minion");
    l[at] = l[at].replace("value=22", &format!("value={named}"));
    let turn = l
        .iter()
        .position(|x| x.contains("tag=TURN value=2"))
        .expect("combat turn");
    l.splice(turn..turn, extra);
    l
}

fn own_power(r: &GameReport) -> bg_parser::report::KitCard {
    r.rounds[0]
        .entries
        .iter()
        .find(|e| e.side == Side::Own)
        .and_then(|e| e.kit.clone())
        .expect("kit")
        .powers
        .remove(0)
}

#[test]
fn a_minion_the_power_marks_is_its_target_when_the_named_one_is_not_shown() {
    // A Duos teammate's board is made of copies: the power names the
    // original (never shown) and marks the copy with its enchantment.
    let mark = entity(
        1,
        70,
        "SYN_POWERe",
        &[
            ("CARDTYPE", "ENCHANTMENT".into()),
            ("CONTROLLER", "2".into()),
            ("ZONE", "PLAY".into()),
            ("ATTACHED", "20".into()),
        ],
    );
    let r = report(&solo_naming(999, mark));
    let power = own_power(&r);
    assert_eq!(power.target, Some(1));
    assert_eq!(power.held, None);
    // Another card's enchantment is no mark.
    let other = entity(
        1,
        70,
        "SYN_OTHERe",
        &[
            ("CARDTYPE", "ENCHANTMENT".into()),
            ("CONTROLLER", "2".into()),
            ("ZONE", "PLAY".into()),
            ("ATTACHED", "20".into()),
        ],
    );
    assert_eq!(own_power(&report(&solo_naming(999, other))).target, None);
}

#[test]
fn a_minion_the_power_holds_off_the_board_is_kept_with_its_stats() {
    let held = entity(
        1,
        71,
        "SYN_HELD",
        &[
            ("CARDTYPE", "MINION".into()),
            ("CONTROLLER", "2".into()),
            ("ZONE", "SETASIDE".into()),
            ("ATK", "6".into()),
            ("HEALTH", "10".into()),
            ("TAUNT", "1".into()),
        ],
    );
    let r = report(&solo_naming(71, held));
    let power = own_power(&r);
    let held = power.held.expect("held minion");
    assert_eq!(
        (held.card_id.as_deref(), held.atk, held.health),
        (Some("SYN_HELD"), 6, 10)
    );
    assert!(held.keywords.contains(&"taunt"));
    assert_eq!(power.target, None);
    // A minion on the board is a target, not held.
    assert_eq!(own_power(&report(&solo())).held, None);
    // Saved with the player's own kit (parser revision 16): its card id,
    // stats, golden flag and keywords.
    let json = serde_json::to_value(&r).expect("serializable");
    let held = &json["rounds"][0]["entries"][0]["kit"]["powers"][0]["held"];
    assert_eq!(
        held,
        &serde_json::json!({
            "card_id": "SYN_HELD", "atk": 6, "health": 10, "golden": false, "keywords": ["taunt"]
        })
    );
}

/// `solo_naming(71, ...)` with the held minion carrying a Dark Gift, for
/// the saved report below.
fn solo_holding_a_gifted_minion() -> Vec<String> {
    let mut extra = entity(
        1,
        71,
        "SYN_HELD",
        &[
            ("CARDTYPE", "MINION".into()),
            ("CONTROLLER", "2".into()),
            ("ZONE", "SETASIDE".into()),
            ("ATK", "6".into()),
            ("HEALTH", "10".into()),
        ],
    );
    extra.extend(entity(
        1,
        72,
        "BG36_MidGameEffect_000t9e2",
        &[
            ("CARDTYPE", "ENCHANTMENT".into()),
            ("CONTROLLER", "2".into()),
            ("ZONE", "PLAY".into()),
            ("ATTACHED", "71".into()),
        ],
    ));
    let mut l = solo_naming(71, extra);
    // The opponent's minion shows its two numbers (as Hopebringer does).
    let at = l
        .iter()
        .position(|x| x.contains("Entity=40 EffectCardId"))
        .expect("the opponent's start of combat");
    l.splice(
        at..at,
        [
            tag("40", "TAG_SCRIPT_DATA_NUM_1", "56"),
            tag("40", "TAG_SCRIPT_DATA_NUM_2", "42"),
        ],
    );
    l
}

/// The parser's report of a game with each side's kit and a held minion,
/// saved for upload-game's Deno tests and the key contract (parser revision
/// 16). `BLESS=1 cargo test -p bg-parser --test start_of_combat` writes it
/// again.
#[test]
fn the_saved_report_with_kits_for_upload_game_is_current() {
    let reports = vec![report(&solo_holding_a_gifted_minion()), report(&solo())];
    let text = serde_json::to_string_pretty(&reports).unwrap() + "\n";
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/rust/kit_games.json");
    if std::env::var_os("BLESS").is_some() {
        std::fs::write(&path, &text).unwrap();
    }
    let saved = std::fs::read_to_string(&path)
        .unwrap_or_default()
        .replace("\r\n", "\n");
    assert_eq!(
        saved, text,
        "run BLESS=1 cargo test -p bg-parser --test start_of_combat"
    );
    assert!(
        text.contains("\"held\"") && text.contains("\"target\"") && text.contains("\"counters\"")
    );
}

#[test]
fn each_sides_kit_is_saved_with_its_board() {
    let r = report(&solo());
    let json = serde_json::to_value(&r).expect("serializable");
    let entries = &json["rounds"][0]["entries"];
    assert_eq!(entries[0]["side"], "own");
    assert_eq!(
        entries[0]["kit"],
        serde_json::json!({
            "powers": [{"card_id": "SYN_POWER", "slot": 0, "counters": [5, 0, 0, 0, 0], "target": 2}],
            "trinkets": [
                {"card_id": "SYN_LESSER", "slot": 1, "counters": [0, 3, 0, 0, 0]},
                {"card_id": "BG30_Trinket_2nd", "slot": 2, "counters": [4, 0, 0, 0, 0]}
            ]
        })
    );
    // The opponent's too (D-164), as the game shows them at combat setup.
    const { assert!(bg_parser::report::SAVE_OPPONENT_KIT) };
    assert_eq!(
        entries[1]["kit"],
        serde_json::json!({
            "powers": [{"card_id": "SYN_FOE_POWER", "slot": 0, "counters": [2, 0, 0, 0, 0]}],
            "trinkets": [{"card_id": "SYN_GREATER", "slot": 2}]
        })
    );
    assert!(!json.to_string().contains("before"));
}

#[test]
fn the_opponents_kit_is_saved_without_a_held_minion() {
    let mut r = report(&solo());
    let held = report(&solo_holding_a_gifted_minion()).rounds[0].entries[0]
        .kit
        .clone()
        .expect("kit")
        .powers[0]
        .held
        .clone();
    assert!(held.is_some());
    let opp = r.rounds[0]
        .entries
        .iter_mut()
        .find(|e| e.side == Side::Opponent)
        .unwrap();
    opp.kit.as_mut().unwrap().powers[0].held = held;
    let json = serde_json::to_value(&r).expect("serializable");
    assert_eq!(
        json["rounds"][0]["entries"][1]["kit"]["powers"][0].get("held"),
        None
    );
    assert!(!json.to_string().contains("SYN_HELD"));
}

#[test]
fn a_combat_without_start_of_combat_effects_has_the_same_board_before() {
    let mut l = solo();
    let at = l.iter().position(|x| x.contains("Entity=40 ")).unwrap();
    // Drop the opponent's start-of-combat block (4 lines).
    l.drain(at..at + 4);
    let r = report(&l);
    for e in &r.rounds[0].entries {
        assert_eq!(e.before, e.board, "{:?}", e.side);
    }
}

/// A Duos combat: the player's first board fights, dies, and the teammate's
/// board comes in with its own hero, hero power and start of combat.
fn duos() -> Vec<String> {
    let mut l = header("GT_BATTLEGROUNDS_DUO");
    l.extend(hero(102, "SYN_HERO_A", 2, 2, "PLAY"));
    l.extend(hero(21, "SYN_HERO_B", 10, 3, "SETASIDE"));
    l.push(tag("2", "HERO_ENTITY", "102"));
    l.push(tag("GameEntity", "TURN", "1"));
    l.extend(minion(20, "SYN_OWN_A", 2, 1, 1, 1));
    l.push(tag("2", "RESOURCES", "1"));
    l.push(tag("GameEntity", "TURN", "2"));
    l.extend(hero(44, "SYN_HERO_B", 10, 0, "PLAY"));
    l.push(tag("3", "HERO_ENTITY", "44"));
    l.extend(minion(40, "SYN_FOE_A", 10, 1, 5, 5));
    l.push(block("ATTACK", "20", 0));
    l.push(tag("GameEntity", "PROPOSED_DEFENDER", "40"));
    l.push(tag("40", "DAMAGE", "1"));
    l.push(tag("20", "DAMAGE", "5"));
    l.push(end(0));
    l.push(tag("20", "ZONE", "GRAVEYARD"));
    // The teammate's board comes in.
    l.extend(hero(103, "SYN_MATE", 2, 0, "PLAY"));
    l.push(tag("2", "HERO_ENTITY", "103"));
    l.extend(kit_card(
        55,
        "SYN_MATE_POWER",
        "HERO_POWER",
        2,
        &[("TAG_SCRIPT_DATA_NUM_1", 1)],
    ));
    l.extend(minion(80, "SYN_MATE_A", 2, 1, 3, 3));
    l.push(block("TRIGGER", "80", 0));
    l.push(tag("80", "ATK", "4"));
    l.push(end(0));
    l.push(block("ATTACK", "80", 0));
    l.push(tag("GameEntity", "PROPOSED_DEFENDER", "40"));
    l.push(tag("40", "DAMAGE", "5"));
    l.push(tag("80", "DAMAGE", "5"));
    l.push(end(0));
    l.push(tag("GameEntity", "TURN", "3"));
    l.push(tag("GameEntity", "STATE", "COMPLETE"));
    l
}

#[test]
fn a_duos_board_that_comes_in_is_kept_as_it_came_in() {
    let r = report(&duos());
    let own: Vec<_> = r.rounds[0]
        .entries
        .iter()
        .filter(|e| e.side == Side::Own)
        .collect();
    assert_eq!(own.len(), 2, "{:?}", r.rounds[0].entries);
    let mate = own[1];
    assert_eq!(mate.hero.as_deref(), Some("SYN_MATE"));
    assert_eq!(
        stats(mate.board.as_deref().unwrap()),
        [("SYN_MATE_A", 4, 3)]
    );
    assert_eq!(
        stats(mate.before.as_deref().expect("as it came in")),
        [("SYN_MATE_A", 3, 3)]
    );
    let kit = mate.kit.clone().expect("the teammate's kit");
    assert_eq!(kit.powers[0].card_id, "SYN_MATE_POWER");
    // The first board had no start-of-combat effect.
    assert_eq!(own[0].before, own[0].board);
}

/// [`solo`] with `extra` just before the line that holds `needle`.
fn solo_with(needle: &str, extra: Vec<String>) -> Vec<String> {
    let mut l = solo();
    let at = l.iter().position(|x| x.contains(needle)).expect("needle");
    l.splice(at..at, extra);
    l
}

#[test]
fn an_effect_from_a_hand_or_a_secret_is_counted_but_never_on_a_board() {
    // The opponent's hand card and Secret fire at the start of combat; the
    // Old God in the Secret zone is on screen and not counted.
    let mut extra = entity(
        1,
        70,
        "SYN_HAND",
        &[
            ("CARDTYPE", "MINION".into()),
            ("CONTROLLER", "10".into()),
            ("ZONE", "HAND".into()),
        ],
    );
    for (id, card) in [(71, "SYN_SECRET"), (72, "BG_OldGod")] {
        extra.extend(entity(
            1,
            id,
            card,
            &[
                ("CARDTYPE", "SPELL".into()),
                ("CONTROLLER", "10".into()),
                ("ZONE", "SECRET".into()),
            ],
        ));
    }
    for id in [70, 71, 72] {
        extra.push(block("TRIGGER", &id.to_string(), 0));
        extra.push(end(0));
    }
    let r = report(&solo_with("Entity=40 EffectCardId", extra));
    assert_eq!(
        r.rounds[0].hidden_triggers,
        [
            (Side::Opponent, "SYN_HAND".to_string()),
            (Side::Opponent, "SYN_SECRET".to_string())
        ]
    );
    let opp = r.rounds[0]
        .entries
        .iter()
        .find(|e| e.side == Side::Opponent)
        .unwrap();
    let before: Vec<_> = opp
        .before
        .iter()
        .flatten()
        .filter_map(|m| m.card_id.as_deref())
        .collect();
    assert_eq!(before, ["SYN_FOE_A", "SYN_FOE_B"]);
    let json = serde_json::to_string(&r).expect("serializable");
    assert!(!json.contains("SYN_HAND") && !json.contains("hidden"));
}

#[test]
fn a_card_that_fires_before_the_opponent_is_set_up_does_not_take_the_boards() {
    // The player's minion fires before the game sets the opponent up: the
    // boards are taken later, at the opponent's own start of combat.
    let extra = vec![tag("20", "ATK", "9"), block("TRIGGER", "20", 0), end(0)];
    let r = report(&solo_with("BlockType=TRIGGER Entity=60", extra));
    let opp = r.rounds[0]
        .entries
        .iter()
        .find(|e| e.side == Side::Opponent)
        .unwrap();
    assert_eq!(
        stats(opp.before.as_deref().expect("taken")),
        [("SYN_FOE_A", 4, 4), ("SYN_FOE_B", 1, 2)]
    );
}

/// The owner's own Solo game (build 253216), round 4: the opponent's Flighty
/// Scout fired from the hand at the start of combat. The board before it has
/// three minions, the one at the first attack four.
#[test]
fn a_real_combat_keeps_the_board_before_a_hand_summon() {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/real");
    let text = std::fs::read_to_string(dir.join("b253216_solo.log")).expect("fixture");
    let reports = bg_parser::parse_reader(text.as_bytes()).expect("read");
    let round = reports[0]
        .rounds
        .iter()
        .find(|r| r.number == 4)
        .expect("round 4");
    let opp = round
        .entries
        .iter()
        .find(|e| e.side == Side::Opponent && e.board.is_some())
        .unwrap();
    let ids = |b: &[bg_parser::report::Minion]| -> Vec<String> {
        b.iter().filter_map(|m| m.card_id.clone()).collect()
    };
    let after = ids(opp.board.as_deref().unwrap());
    let before = ids(opp.before.as_deref().unwrap());
    assert_eq!((before.len(), after.len()), (3, 4), "{before:?} {after:?}");
    assert!(after.iter().any(|c| c == "BG32_330") && !before.iter().any(|c| c == "BG32_330"));
    assert!(round
        .hidden_triggers
        .contains(&(Side::Opponent, "BG32_330".to_string())));
    assert!(opp.kit.is_some());
}

#[test]
fn a_minion_keeps_the_numbers_its_text_shows_and_saves_them_on_the_replay_only() {
    // Like Hopebringer's growing buff: TAG_SCRIPT_DATA_NUM_1 and _2.
    let extra = vec![
        tag("40", "TAG_SCRIPT_DATA_NUM_1", "56"),
        tag("40", "TAG_SCRIPT_DATA_NUM_2", "42"),
    ];
    let r = report(&solo_with("Entity=40 EffectCardId", extra));
    let opp = r.rounds[0]
        .entries
        .iter()
        .find(|e| e.side == Side::Opponent)
        .unwrap();
    assert_eq!(opp.before.as_ref().unwrap()[0].counters, [56, 42]);
    assert_eq!(opp.before.as_ref().unwrap()[1].counters, [0, 0]);
    // The replay starts with the boards before start of combat: the
    // minion's numbers are saved there (parser revision 16), not on the
    // board at the first attack.
    let json = serde_json::to_value(&r).expect("serializable");
    let round = &json["rounds"][0];
    let minions = round["replay"]["minions"].as_array().expect("replay");
    let foe = minions
        .iter()
        .find(|m| m["card_id"] == "SYN_FOE_A")
        .expect("in the replay");
    assert_eq!(foe["counters"], serde_json::json!([56, 42]));
    assert!(minions
        .iter()
        .filter(|m| m["card_id"] != "SYN_FOE_A")
        .all(|m| m.get("counters").is_none()));
    let boards = round["entries"].to_string();
    assert!(!boards.contains("\"counters\":[56"));
}

/// `solo()` with `extra` lines added before the combat turn.
fn solo_plus(extra: Vec<String>) -> Vec<String> {
    let mut l = solo();
    let turn = l
        .iter()
        .position(|x| x.contains("tag=TURN value=2"))
        .expect("combat turn");
    l.splice(turn..turn, extra);
    l
}

#[test]
fn slots_and_targets_are_kept_only_within_what_a_board_has() {
    // A slot no trinket has, and a named minion past the seventh place.
    let r = report(&solo_plus(vec![
        tag("51", "TAG_SCRIPT_DATA_NUM_6", "9"),
        tag("22", "ZONE_POSITION", "9"),
    ]));
    let kit = r.rounds[0].entries[0].kit.clone().expect("kit");
    assert_eq!(kit.trinkets[0].card_id, "SYN_LESSER");
    assert_eq!(kit.trinkets[0].slot, 0);
    assert_eq!(kit.powers[0].target, None);
}

#[test]
fn a_trinket_never_names_or_holds_a_minion() {
    let held = entity(
        1,
        71,
        "SYN_HELD",
        &[
            ("CARDTYPE", "MINION".into()),
            ("CONTROLLER", "2".into()),
            ("ZONE", "SETASIDE".into()),
            ("ATK", "6".into()),
            ("HEALTH", "10".into()),
        ],
    );
    for named in ["22", "71"] {
        let mut extra = held.clone();
        extra.push(tag("51", "TAG_SCRIPT_DATA_ENT_1", named));
        let r = report(&solo_plus(extra));
        let trinket = r.rounds[0].entries[0].kit.clone().expect("kit").trinkets[0].clone();
        assert_eq!(trinket.card_id, "SYN_LESSER");
        assert_eq!((trinket.target, trinket.held), (None, None), "{named}");
    }
}
