//! What the combat simulator reads from the parser (T-603b, T-604, T-605):
//! each combat's attacks in order with both boards before each one, the
//! lasting keywords the saved report leaves out, and the own board in a shop
//! turn. Synthetic log written out below; nothing here reaches the saved
//! report's JSON.

use bg_parser::report::{DefenderSource, GameReport, Side};
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
