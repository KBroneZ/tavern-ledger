//! The combat replay (T-215, D-099, parser revision 8): each combat the game
//! plays back, event by event, on a synthetic log written out below, and the
//! user's two real games (tests/data/real) checked against what the report
//! already says about each combat.

use bg_parser::replay::{
    Cause, CombatEvent, CombatReplay, DamageSource, TriggerKind, MAX_REPLAY_EVENTS,
};

/// A damage put down to replay minion `minion`, as `how`.
fn by(how: Cause, minion: usize) -> Option<DamageSource> {
    Some(DamageSource {
        how,
        minion: Some(minion),
        card_id: None,
        side: None,
    })
}
use bg_parser::report::{CombatResult, GameReport, Side};
use bg_parser::LogReader;

const T: &str = "D 12:00:00.0000000 GameState.DebugPrintPower() - ";

fn line(indent: usize, body: &str) -> String {
    format!("{T}{}{body}", "    ".repeat(indent))
}

fn entity(id: i64, card: &str, tags: &[(&str, String)]) -> Vec<String> {
    let mut out = vec![line(
        1,
        &format!("FULL_ENTITY - Creating ID={id} CardID={card}"),
    )];
    for (tag, value) in tags {
        out.push(line(2, &format!("tag={tag} value={value}")));
    }
    out.push(line(2, &format!("tag=ENTITY_ID value={id}")));
    out
}

fn minion(
    id: i64,
    card: &str,
    controller: i64,
    pos: i64,
    stats: (i64, i64),
    kw: &[&str],
) -> Vec<String> {
    let mut tags = vec![
        ("CARDTYPE", "MINION".to_string()),
        ("CONTROLLER", controller.to_string()),
        ("ZONE", "PLAY".to_string()),
        ("ATK", stats.0.to_string()),
        ("HEALTH", stats.1.to_string()),
        ("ZONE_POSITION", pos.to_string()),
    ];
    tags.extend(kw.iter().map(|k| (*k, "1".to_string())));
    entity(id, card, &tags)
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
    entity(id, card, &tags)
}

fn tag(entity: &str, tag: &str, value: &str) -> String {
    line(
        1,
        &format!("TAG_CHANGE Entity={entity} tag={tag} value={value} "),
    )
}

fn block(kind: &str, entity: i64, keyword: &str) -> String {
    line(
        0,
        &format!(
            "BLOCK_START BlockType={kind} Entity={entity} EffectCardId=System.Collections.Generic.List`1[System.String] EffectIndex=0 Target=0 SubOption=-1 TriggerKeyword={keyword}"
        ),
    )
}

fn end() -> String {
    line(0, "BLOCK_END")
}

/// A Solo game, local player 2 against the shop's player 10, up to its
/// first combat's first line, with the combat set up.
fn start() -> Vec<String> {
    let mut l = combat_turn();
    l.push(tag("20", "ZONE", "SETASIDE"));
    l.extend(hero(44, "SYN_HERO_B", 10, 0, "PLAY"));
    l.push(tag("3", "HERO_ENTITY", "44"));
    l.push(tag("GameEntity", "NEXT_STEP", "MAIN_ACTION"));
    l
}

/// The same game up to the first combat turn, the shop board still in play.
fn combat_turn() -> Vec<String> {
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
    l.extend(hero(102, "SYN_HERO_A", 2, 2, "PLAY"));
    l.extend(hero(21, "SYN_HERO_B", 10, 3, "SETASIDE"));
    l.push(tag("2", "HERO_ENTITY", "102"));
    l.push(tag("GameEntity", "TURN", "1"));
    // The shop board: it waits set aside during the combat and comes back
    // as it ends, never a summon.
    l.extend(minion(20, "SYN_SHOP", 2, 1, (1, 1), &[]));
    l.push(tag("2", "RESOURCES", "3"));
    l.push(tag("GameEntity", "TURN", "2"));
    l
}

/// Round 1: a start-of-combat trigger, two attacks with a Divine Shield
/// popped and two deaths, a deathrattle that summons a buffed token, which
/// kills the last enemy; the hero hits for 5; then the game cleans up.
fn won_combat() -> Vec<String> {
    let mut l = start();
    l.extend(minion(30, "SYN_SHIELD", 2, 1, (2, 3), &["DIVINE_SHIELD"]));
    l.extend(minion(31, "SYN_DR", 2, 2, (1, 1), &["DEATHRATTLE"]));
    l.extend(minion(40, "SYN_FOE_A", 10, 1, (3, 2), &[]));
    l.extend(minion(41, "SYN_FOE_B_G", 10, 2, (1, 4), &["TAUNT"]));
    l.push(block("TRIGGER", 31, "TAG_NOT_SET"));
    l.push(tag("41", "DAMAGE", "1"));
    l.push(end());
    l.push(block("ATTACK", 30, ""));
    l.push(tag("GameEntity", "PROPOSED_DEFENDER", "40"));
    l.push(tag("40", "DAMAGE", "2"));
    l.push(tag("30", "DIVINE_SHIELD", "0"));
    l.push(end());
    l.push(tag("40", "ZONE", "GRAVEYARD"));
    l.push(tag("40", "DAMAGE", "0"));
    l.push(block("ATTACK", 41, ""));
    l.push(tag("31", "DAMAGE", "1"));
    l.push(tag("41", "DAMAGE", "2"));
    l.push(end());
    l.push(tag("31", "ZONE", "GRAVEYARD"));
    l.push(block("TRIGGER", 31, "DEATHRATTLE"));
    l.extend(minion(50, "SYN_TOKEN", 2, 2, (1, 1), &[]));
    l.push(end());
    l.push(tag("50", "ATK", "3"));
    l.push(tag("50", "HEALTH", "2"));
    l.push(block("ATTACK", 50, ""));
    l.push(tag("GameEntity", "PROPOSED_DEFENDER", "41"));
    l.push(tag("41", "DAMAGE", "5"));
    l.push(tag("50", "DAMAGE", "1"));
    l.push(end());
    l.push(tag("41", "ZONE", "GRAVEYARD"));
    l.push(tag("102", "ATK", "5"));
    l.push(block("ATTACK", 102, ""));
    l.push(tag("44", "DAMAGE", "5"));
    l.push(end());
    // Clean-up: copies go, the shop board comes back.
    l.push(tag("30", "ZONE", "REMOVEDFROMGAME"));
    l.push(tag("50", "ATK", "9"));
    l.push(tag("20", "ZONE", "PLAY"));
    l
}

fn read(lines: &[String]) -> GameReport {
    let mut reader = LogReader::default();
    lines.iter().for_each(|l| reader.feed(l));
    let mut reports = reader.finish();
    assert_eq!(reports.len(), 1);
    reports.remove(0)
}

fn finished(mut lines: Vec<String>) -> GameReport {
    lines.push(tag("GameEntity", "TURN", "3"));
    lines.push(tag("GameEntity", "STATE", "COMPLETE"));
    read(&lines)
}

fn replay(r: &GameReport, round: i64) -> CombatReplay {
    let round = r.rounds.iter().find(|x| x.number == round).expect("round");
    round.replay.clone().expect("a shown combat has a replay")
}

#[test]
fn a_shown_combat_keeps_every_event_in_the_logs_order() {
    let r = finished(won_combat());
    let replay = replay(&r, 1);
    assert!(replay.complete);
    assert_eq!(replay.start, 4, "both boards before the first trigger");
    let cards: Vec<_> = replay
        .minions
        .iter()
        .map(|m| m.card_id.as_deref().unwrap())
        .collect();
    assert_eq!(
        cards,
        [
            "SYN_SHIELD",
            "SYN_DR",
            "SYN_FOE_A",
            "SYN_FOE_B_G",
            "SYN_TOKEN"
        ]
    );
    assert_eq!(replay.minions[0].side, Side::Own);
    assert_eq!(replay.minions[2].side, Side::Opponent);
    assert_eq!(
        replay.minions[3].health, 4,
        "health before the trigger's damage"
    );
    assert!(replay.minions[3].golden && replay.minions[3].keywords == ["taunt"]);
    assert_eq!(
        (replay.minions[4].position, replay.minions[4].health),
        (2, 1)
    );
    use CombatEvent::*;
    assert_eq!(
        replay.events,
        [
            Trigger {
                trigger: TriggerKind::StartOfCombat,
                side: Side::Own,
                card_id: "SYN_DR".into(),
                source: Some(1),
            },
            Damage {
                minion: 3,
                amount: 1,
                health: 3,
                source: by(Cause::StartOfCombat, 1),
            },
            Attack {
                attacker: 0,
                defender: Some(2)
            },
            Damage {
                minion: 2,
                amount: 2,
                health: 0,
                source: by(Cause::Attack, 0),
            },
            Shield {
                minion: 0,
                on: false
            },
            Death { minion: 2 },
            // No PROPOSED_DEFENDER: the first enemy minion hit.
            Attack {
                attacker: 3,
                defender: Some(1)
            },
            Damage {
                minion: 1,
                amount: 1,
                health: 0,
                source: by(Cause::Attack, 3),
            },
            Damage {
                minion: 3,
                amount: 1,
                health: 2,
                source: by(Cause::Attack, 1),
            },
            Death { minion: 1 },
            Trigger {
                trigger: TriggerKind::Deathrattle,
                side: Side::Own,
                card_id: "SYN_DR".into(),
                source: Some(1),
            },
            Summon { minion: 4 },
            // Attack then health changed: one event with both.
            Stats {
                minion: 4,
                atk: 3,
                health: 2
            },
            Attack {
                attacker: 4,
                defender: Some(3)
            },
            Damage {
                minion: 3,
                amount: 3,
                health: -1,
                source: by(Cause::Attack, 4),
            },
            Damage {
                minion: 4,
                amount: 1,
                health: 1,
                source: by(Cause::Attack, 3),
            },
            Death { minion: 3 },
            HeroDamage {
                side: Side::Opponent,
                amount: 5
            },
        ]
    );
    assert_eq!(r.rounds[0].result, None, "no result tags in this log");
}

/// An enchantment in play on `on`, as the log creates it.
fn enchantment(id: i64, card: &str, on: i64) -> Vec<String> {
    entity(
        id,
        card,
        &[
            ("CARDTYPE", "ENCHANTMENT".to_string()),
            ("ATTACHED", on.to_string()),
            ("ZONE", "PLAY".to_string()),
        ],
    )
}

#[test]
fn a_replay_minion_keeps_its_buffs_from_the_start_and_when_summoned() {
    // T-619, parser revision 14: the buffs a minion has as the combat starts,
    // and those a token has as it is summoned, as on a board minion.
    let mut l = won_combat();
    let at = l
        .iter()
        .position(|x| x.contains("BlockType=TRIGGER"))
        .expect("first block");
    l.splice(
        at..at,
        [
            enchantment(70, "BG20_GEMe2", 30),
            enchantment(71, "BG20_GEMe2", 30),
            enchantment(72, "BG31_816e", 41),
        ]
        .concat(),
    );
    let at = l
        .iter()
        .position(|x| x.contains("Creating ID=50"))
        .expect("token");
    // The token's own lines end at its ENTITY_ID; its buff follows them.
    l.splice(at + 8..at + 8, enchantment(73, "BG28_309e", 50));
    let r = replay(&finished(l), 1);
    let buffs = |card: &str| -> Vec<(String, u32)> {
        r.minions
            .iter()
            .find(|m| m.card_id.as_deref() == Some(card))
            .expect(card)
            .buffs
            .iter()
            .map(|b| (b.card_id.clone(), b.count))
            .collect()
    };
    assert_eq!(buffs("SYN_SHIELD"), vec![("BG20_GEM".to_string(), 2)]);
    assert_eq!(buffs("SYN_FOE_B_G"), vec![("BG31_816".to_string(), 1)]);
    assert!(buffs("SYN_DR").is_empty());
    assert_eq!(buffs("SYN_TOKEN"), vec![("BG28_309".to_string(), 1)]);
}

#[test]
fn a_summoned_minion_keeps_its_buffs_when_it_dies_and_they_are_taken_away() {
    // The log moves the minion to the graveyard, then each of its
    // enchantments out of the game: the saved buffs are what it had alive.
    let mut l = won_combat();
    let at = l
        .iter()
        .position(|x| x.contains("Creating ID=50"))
        .expect("token");
    l.splice(at + 8..at + 8, enchantment(73, "BG28_309e", 50));
    l.splice(at + 8 + 7..at + 8 + 7, enchantment(74, "BG28_309e", 50));
    // In the combat: right after the token's attack, before the last enemy dies.
    let end = l
        .iter()
        .position(|x| x.contains("Entity=41 tag=ZONE value=GRAVEYARD"))
        .expect("the last enemy dies");
    l.splice(
        end..end,
        [
            tag("50", "ZONE", "GRAVEYARD"),
            tag("73", "ZONE", "REMOVEDFROMGAME"),
            tag("74", "ZONE", "REMOVEDFROMGAME"),
        ],
    );
    let r = replay(&finished(l), 1);
    let token = r
        .minions
        .iter()
        .find(|m| m.card_id.as_deref() == Some("SYN_TOKEN"))
        .expect("token");
    let kinds: Vec<_> = token
        .buffs
        .iter()
        .map(|b| (b.card_id.as_str(), b.count))
        .collect();
    assert_eq!(kinds, [("BG28_309", 2)]);
}

#[test]
fn the_replay_is_saved_with_its_round() {
    let r = finished(won_combat());
    let json = serde_json::to_value(&r).expect("serializable");
    let replay = &json["rounds"][0]["replay"];
    assert_eq!(replay["start"], 4);
    assert_eq!(replay["complete"], true);
    assert_eq!(
        replay["minions"][4],
        serde_json::json!({
            "side": "own", "card_id": "SYN_TOKEN", "atk": 1, "health": 1,
            "position": 2, "golden": false, "keywords": []
        })
    );
    assert_eq!(
        replay["events"][0],
        serde_json::json!({
            "kind": "trigger", "trigger": "start_of_combat", "side": "own",
            "card_id": "SYN_DR", "source": 1
        })
    );
    assert_eq!(
        replay["events"][2],
        serde_json::json!({ "kind": "attack", "attacker": 0, "defender": 2 })
    );
    assert_eq!(
        replay["events"][17],
        serde_json::json!({ "kind": "hero_damage", "side": "opponent", "amount": 5 })
    );
}

#[test]
fn a_combat_the_game_does_not_play_back_has_no_replay() {
    let mut l = won_combat();
    l.push(tag("GameEntity", "TURN", "3"));
    l.push(tag("GameEntity", "TURN", "4"));
    // An opponent steps in, but no attack is ever logged.
    l.extend(hero(45, "SYN_HERO_C", 10, 0, "PLAY"));
    l.push(tag("3", "HERO_ENTITY", "45"));
    l.extend(minion(60, "SYN_HIDDEN", 10, 1, (5, 5), &[]));
    l.push(block("TRIGGER", 60, "TAG_NOT_SET"));
    l.push(end());
    l.push(tag("GameEntity", "TURN", "5"));
    l.push(tag("GameEntity", "STATE", "COMPLETE"));
    let r = read(&l);
    let hidden = r.rounds.iter().find(|x| x.number == 2).expect("round 2");
    assert_eq!(hidden.entries[0].board, None, "not visible");
    assert_eq!(hidden.replay, None, "not shown");
    let json = serde_json::to_value(&r).expect("serializable");
    assert!(
        json["rounds"][1]["replay"].is_null(),
        "saved as null, never left out"
    );
}

#[test]
fn a_log_cut_during_a_combat_gives_an_incomplete_replay() {
    let all = won_combat();
    let cut = all
        .iter()
        .position(|l| l.contains("Entity=50 "))
        .expect("token attack");
    let r = read(&all[..cut]);
    let replay = replay(&r, 1);
    assert!(!replay.complete, "the combat never ended in the log");
    assert!(replay.events.contains(&CombatEvent::Summon { minion: 4 }));
}

#[test]
fn duos_a_board_coming_in_keeps_the_players_own_minions() {
    let mut l = start();
    // The teammate's board fights first and dies.
    l.extend(minion(30, "SYN_MATE", 2, 1, (1, 1), &[]));
    l.extend(minion(40, "SYN_FOE_A", 10, 1, (5, 5), &[]));
    l.push(block("ATTACK", 30, ""));
    l.push(tag("GameEntity", "PROPOSED_DEFENDER", "40"));
    l.push(tag("30", "DAMAGE", "5"));
    l.push(end());
    l.push(tag("30", "ZONE", "GRAVEYARD"));
    // The player's own board comes in: its minion was made before the
    // combat (the shop's), not a copy.
    l.extend(hero(103, "SYN_HERO_OWN", 2, 0, "PLAY"));
    l.push(tag("2", "HERO_ENTITY", "103"));
    l.push(tag("20", "ZONE", "PLAY"));
    l.push(tag("20", "ZONE_POSITION", "1"));
    l.push(block("ATTACK", 40, ""));
    l.push(tag("GameEntity", "PROPOSED_DEFENDER", "20"));
    l.push(tag("20", "DAMAGE", "5"));
    l.push(end());
    l.push(tag("20", "ZONE", "GRAVEYARD"));
    l.push(tag("44", "ATK", "4"));
    l.push(block("ATTACK", 44, ""));
    l.push(end());
    let r = finished(l);
    let replay = replay(&r, 1);
    use CombatEvent::*;
    assert_eq!(
        replay.events[3..],
        [
            BoardIn {
                side: Side::Own,
                hero: Some("SYN_HERO_OWN".into())
            },
            Summon { minion: 2 },
            Attack {
                attacker: 1,
                defender: Some(2)
            },
            Damage {
                minion: 2,
                amount: 5,
                health: -4,
                source: by(Cause::Attack, 1),
            },
            Death { minion: 2 },
            HeroDamage {
                side: Side::Own,
                amount: 4
            },
        ]
    );
    assert_eq!(replay.minions[2].card_id.as_deref(), Some("SYN_SHOP"));
}

#[test]
fn a_tie_drops_the_heroes_and_boards_the_game_puts_back() {
    let mut l = start();
    l.extend(minion(30, "SYN_OWN", 2, 1, (2, 2), &[]));
    l.extend(minion(40, "SYN_FOE", 10, 1, (2, 2), &[]));
    l.push(block("ATTACK", 30, ""));
    l.push(tag("GameEntity", "PROPOSED_DEFENDER", "40"));
    l.push(tag("40", "DAMAGE", "2"));
    l.push(tag("30", "DAMAGE", "2"));
    l.push(end());
    l.push(tag("30", "ZONE", "GRAVEYARD"));
    l.push(tag("40", "ZONE", "GRAVEYARD"));
    // A tie: no hero attack. The game swaps the hero back and returns the
    // shop board while the combat turn is still on.
    l.extend(hero(103, "SYN_HERO_OWN", 2, 0, "PLAY"));
    l.push(tag("2", "HERO_ENTITY", "103"));
    l.push(tag("20", "ZONE", "PLAY"));
    let r = finished(l);
    let replay = replay(&r, 1);
    assert_eq!(replay.minions.len(), 2, "the shop minion is not a summon");
    assert!(matches!(
        replay.events.last(),
        Some(CombatEvent::Death { minion: 1 })
    ));
    assert!(replay.complete);
}

/// What the report already says about each combat, checked against the
/// replay on the user's two real games: who survives, the hero damage, and
/// that every event names a minion on the board.
#[test]
fn real_games_replays_agree_with_the_rest_of_the_report() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/real");
    let mut shown = 0;
    for name in ["b253216_solo", "b253216_duos"] {
        let text = std::fs::read_to_string(dir.join(format!("{name}.log"))).expect("fixture");
        let reports = bg_parser::parse_reader(text.as_bytes()).expect("read");
        for round in &reports[0].rounds {
            let seen = round.entries.iter().any(|e| e.board.is_some());
            assert_eq!(
                round.replay.is_some(),
                seen,
                "{name} round {}",
                round.number
            );
            let Some(replay) = &round.replay else {
                continue;
            };
            shown += 1;
            assert!(replay.complete, "{name} round {}", round.number);
            let alive = check_events(replay, &format!("{name} round {}", round.number));
            let left = |side| {
                alive
                    .iter()
                    .filter(|&&i| replay.minions[i].side == side)
                    .count()
            };
            let hero = replay.events.iter().find_map(|e| match e {
                CombatEvent::HeroDamage { side, .. } => Some(*side),
                _ => None,
            });
            match round.result {
                Some(CombatResult::Won) => {
                    assert_eq!(left(Side::Opponent), 0);
                    assert_eq!(hero, Some(Side::Opponent));
                }
                Some(CombatResult::Lost) => {
                    assert_eq!(left(Side::Own), 0);
                    assert_eq!(hero, Some(Side::Own));
                }
                Some(CombatResult::Tie) => assert_eq!(hero, None),
                None => {}
            }
        }
    }
    assert_eq!(shown, 17, "every combat of both games is shown");
}

/// Every event names a minion that is on the board then; returns the ones
/// left at the end.
fn check_events(replay: &CombatReplay, what: &str) -> std::collections::BTreeSet<usize> {
    let mut alive: std::collections::BTreeSet<usize> = (0..replay.start).collect();
    for e in &replay.events {
        match e {
            CombatEvent::Summon { minion } => assert!(alive.insert(*minion), "{what}"),
            CombatEvent::Death { minion } => assert!(alive.remove(minion), "{what}"),
            CombatEvent::Attack { attacker, defender } => {
                assert!(alive.contains(attacker), "{what}: attacker");
                let defender = defender.expect("every real attack has its defender");
                assert!(alive.contains(&defender), "{what}: defender");
                assert_ne!(
                    replay.minions[*attacker].side,
                    replay.minions[defender].side
                );
            }
            CombatEvent::Damage { minion, .. }
            | CombatEvent::Shield { minion, .. }
            | CombatEvent::Stats { minion, .. } => assert!(alive.contains(minion), "{what}"),
            _ => {}
        }
    }
    alive
}

#[test]
fn a_combat_past_the_event_bound_stops_and_says_so() {
    let mut l = start();
    l.extend(minion(30, "SYN_BIG", 2, 1, (1, 100_000), &[]));
    l.extend(minion(40, "SYN_FOE", 10, 1, (1, 100_000), &[]));
    l.push(block("ATTACK", 30, ""));
    for damage in 1..=MAX_REPLAY_EVENTS as i64 + 5 {
        l.push(tag("40", "DAMAGE", &damage.to_string()));
    }
    l.push(end());
    // Past the bound: neither the attack nor the summon is kept, and no
    // later hit names a defender for an attack that is not there.
    l.push(block("ATTACK", 40, ""));
    l.push(tag("30", "DAMAGE", "1"));
    l.push(end());
    l.extend(minion(50, "SYN_LATE", 2, 2, (1, 1), &[]));
    let r = finished(l);
    let replay = replay(&r, 1);
    assert!(!replay.complete);
    assert_eq!(replay.events.len(), MAX_REPLAY_EVENTS);
    assert_eq!(replay.minions.len(), 2, "no minion without its summon");
    assert_eq!(
        replay.events[0],
        CombatEvent::Attack {
            attacker: 0,
            defender: Some(1)
        }
    );
}

/// A block nested `depth` levels deep, as the log indents it.
fn nested_block(depth: usize, kind: &str, entity: i64, keyword: &str) -> String {
    line(
        depth,
        &format!(
            "BLOCK_START BlockType={kind} Entity={entity} EffectCardId=System.Collections.Generic.List`1[System.String] EffectIndex=0 Target=0 SubOption=-1 TriggerKeyword={keyword}"
        ),
    )
}

#[test]
fn a_trinket_firing_while_the_game_sets_the_boards_up_does_not_start_the_combat() {
    let mut l = start();
    l.extend(minion(30, "SYN_OWN", 2, 1, (2, 2), &[]));
    // The game sets the opponent's board up in a block of the player
    // entity; their trinket fires inside it before their minions are placed.
    l.push(block("TRIGGER", 2, "TAG_NOT_SET"));
    l.extend(entity(
        70,
        "SYN_TRINKET",
        &[
            ("CARDTYPE", "BATTLEGROUND_TRINKET".to_string()),
            ("CONTROLLER", "10".to_string()),
            ("ZONE", "PLAY".to_string()),
        ],
    ));
    l.push(nested_block(1, "TRIGGER", 70, "TAG_NOT_SET"));
    l.push(line(1, "BLOCK_END"));
    l.extend(minion(40, "SYN_FOE_A", 10, 1, (3, 3), &[]));
    l.extend(minion(41, "SYN_FOE_B", 10, 2, (1, 1), &[]));
    l.push(end());
    l.push(block("ATTACK", 30, ""));
    l.push(tag("GameEntity", "PROPOSED_DEFENDER", "41"));
    l.push(tag("41", "DAMAGE", "2"));
    l.push(end());
    let r = finished(l);
    let replay = replay(&r, 1);
    assert_eq!(replay.start, 3, "both whole boards as the combat starts");
    let cards: Vec<_> = replay
        .minions
        .iter()
        .map(|m| (m.side, m.card_id.as_deref().unwrap()))
        .collect();
    assert_eq!(
        cards,
        [
            (Side::Own, "SYN_OWN"),
            (Side::Opponent, "SYN_FOE_A"),
            (Side::Opponent, "SYN_FOE_B")
        ]
    );
    assert!(
        !replay
            .events
            .iter()
            .any(|e| matches!(e, CombatEvent::Summon { .. })),
        "the board set up is not a summon"
    );
    assert_eq!(
        replay.events[0],
        CombatEvent::Attack {
            attacker: 0,
            defender: Some(2)
        }
    );
}

#[test]
fn a_shop_effect_before_the_combat_is_set_up_does_not_start_it() {
    let mut l = combat_turn();
    l.extend(minion(25, "SYN_SHOP_B", 2, 2, (9, 9), &[]));
    // An end-of-turn effect of the shop board, before the game announces
    // the combat's action step.
    l.push(block("TRIGGER", 25, "1724"));
    l.push(end());
    l.push(tag("20", "ZONE", "SETASIDE"));
    l.push(tag("25", "ZONE", "SETASIDE"));
    l.extend(hero(44, "SYN_HERO_B", 10, 0, "PLAY"));
    l.push(tag("3", "HERO_ENTITY", "44"));
    l.extend(minion(30, "SYN_COPY", 2, 1, (9, 9), &[]));
    l.extend(minion(40, "SYN_FOE_A", 10, 1, (1, 1), &[]));
    l.push(tag("GameEntity", "NEXT_STEP", "MAIN_ACTION"));
    l.push(block("ATTACK", 30, ""));
    l.push(tag("GameEntity", "PROPOSED_DEFENDER", "40"));
    l.push(tag("40", "DAMAGE", "9"));
    l.push(end());
    let r = finished(l);
    let replay = replay(&r, 1);
    let cards: Vec<_> = replay
        .minions
        .iter()
        .map(|m| m.card_id.as_deref().unwrap())
        .collect();
    assert_eq!(
        cards,
        ["SYN_COPY", "SYN_FOE_A"],
        "the combat's boards, not the shop's"
    );
    assert_eq!(replay.start, 2);
    assert_eq!(
        replay.events[0],
        CombatEvent::Attack {
            attacker: 0,
            defender: Some(1)
        }
    );
}

#[test]
fn a_combat_whose_action_step_is_never_announced_still_starts_at_its_first_attack() {
    let mut l = combat_turn();
    // The log announces a step, but not the action step (a log cut and
    // resumed, a reconnect): the triggers wait, the first attack starts it.
    l.push(tag("GameEntity", "NEXT_STEP", "MAIN_READY"));
    l.push(tag("20", "ZONE", "SETASIDE"));
    l.extend(hero(44, "SYN_HERO_B", 10, 0, "PLAY"));
    l.push(tag("3", "HERO_ENTITY", "44"));
    l.extend(minion(30, "SYN_OWN", 2, 1, (2, 2), &[]));
    l.extend(minion(40, "SYN_FOE", 10, 1, (1, 1), &[]));
    l.push(block("TRIGGER", 30, "TAG_NOT_SET"));
    l.push(end());
    l.push(block("ATTACK", 30, ""));
    l.push(tag("GameEntity", "PROPOSED_DEFENDER", "40"));
    l.push(tag("40", "DAMAGE", "2"));
    l.push(end());
    let r = finished(l);
    let replay = replay(&r, 1);
    assert_eq!(replay.start, 2);
    assert_eq!(
        replay.events[0],
        CombatEvent::Attack {
            attacker: 0,
            defender: Some(1)
        }
    );
}

#[test]
fn a_log_without_step_tags_waits_for_the_opponents_hero() {
    // A trimmed fixture keeps no step tags: the shop effect before the
    // opponent's hero is set is the shop's, the trigger after it the combat's.
    let mut l = combat_turn();
    l.extend(minion(25, "SYN_SHOP_B", 2, 2, (9, 9), &[]));
    l.push(block("TRIGGER", 25, "1724"));
    l.push(end());
    l.push(tag("20", "ZONE", "SETASIDE"));
    l.push(tag("25", "ZONE", "SETASIDE"));
    l.extend(hero(44, "SYN_HERO_B", 10, 0, "PLAY"));
    l.push(tag("3", "HERO_ENTITY", "44"));
    l.extend(minion(30, "SYN_COPY", 2, 1, (9, 9), &[]));
    l.extend(minion(40, "SYN_FOE_A", 10, 1, (1, 1), &[]));
    l.push(block("TRIGGER", 30, "TAG_NOT_SET"));
    l.push(end());
    l.push(block("ATTACK", 30, ""));
    l.push(end());
    let r = finished(l);
    let replay = replay(&r, 1);
    let cards: Vec<_> = replay
        .minions
        .iter()
        .map(|m| m.card_id.as_deref().unwrap())
        .collect();
    assert_eq!(cards, ["SYN_COPY", "SYN_FOE_A"]);
    assert!(matches!(
        replay.events[0],
        CombatEvent::Trigger {
            trigger: TriggerKind::StartOfCombat,
            ..
        }
    ));
}

#[test]
fn once_the_combat_has_started_a_nested_trigger_is_kept() {
    let mut l = start();
    l.extend(minion(30, "SYN_OWN", 2, 1, (2, 2), &[]));
    l.extend(minion(40, "SYN_FOE", 10, 1, (1, 3), &["DEATHRATTLE"]));
    l.push(block("ATTACK", 30, ""));
    l.push(tag("GameEntity", "PROPOSED_DEFENDER", "40"));
    l.push(tag("40", "DAMAGE", "3"));
    l.push(nested_block(1, "TRIGGER", 40, "DEATHRATTLE"));
    l.push(line(1, "BLOCK_END"));
    l.push(end());
    let r = finished(l);
    let replay = replay(&r, 1);
    assert!(replay.events.iter().any(|e| matches!(
        e,
        CombatEvent::Trigger {
            trigger: TriggerKind::Deathrattle,
            source: Some(1),
            ..
        }
    )));
}

#[test]
fn each_combat_turn_waits_for_its_own_action_step() {
    let mut l = won_combat();
    l.push(tag("GameEntity", "TURN", "3"));
    l.push(tag("GameEntity", "TURN", "4"));
    // Round 2: a shop effect before this turn's action step is not the
    // combat's, even though round 1 announced its own.
    l.extend(minion(26, "SYN_SHOP_C", 2, 1, (4, 4), &[]));
    l.push(block("TRIGGER", 26, "1724"));
    l.push(end());
    l.push(tag("26", "ZONE", "SETASIDE"));
    l.extend(hero(45, "SYN_HERO_C", 10, 0, "PLAY"));
    l.push(tag("3", "HERO_ENTITY", "45"));
    l.extend(minion(60, "SYN_R2_OWN", 2, 1, (3, 3), &[]));
    l.extend(minion(70, "SYN_R2_FOE", 10, 1, (1, 1), &[]));
    l.push(tag("GameEntity", "NEXT_STEP", "MAIN_ACTION"));
    l.push(block("ATTACK", 60, ""));
    l.push(tag("GameEntity", "PROPOSED_DEFENDER", "70"));
    l.push(tag("70", "DAMAGE", "3"));
    l.push(end());
    l.push(tag("GameEntity", "TURN", "5"));
    l.push(tag("GameEntity", "STATE", "COMPLETE"));
    let r = read(&l);
    let replay = replay(&r, 2);
    let cards: Vec<_> = replay
        .minions
        .iter()
        .map(|m| m.card_id.as_deref().unwrap())
        .collect();
    assert!(!cards.contains(&"SYN_SHOP_C"), "{cards:?}");
    assert!(matches!(replay.events[0], CombatEvent::Attack { .. }));
}

/// A minion the game reveals as it summons it: `SHOW_ENTITY` puts it in
/// play with no slot, and the slot comes in the tag change right after.
fn shown_summon(id: i64, card: &str, controller: i64, slot: i64) -> Vec<String> {
    vec![
        line(1, &format!("FULL_ENTITY - Creating ID={id} CardID=")),
        line(2, "tag=ZONE value=SETASIDE"),
        line(2, &format!("tag=CONTROLLER value={controller}")),
        line(
            1,
            &format!("SHOW_ENTITY - Updating Entity={id} CardID={card}"),
        ),
        line(2, &format!("tag=CONTROLLER value={controller}")),
        line(2, "tag=CARDTYPE value=MINION"),
        line(2, "tag=ATK value=1"),
        line(2, "tag=HEALTH value=1"),
        line(2, "tag=ZONE value=PLAY"),
        tag(&id.to_string(), "ZONE_POSITION", &slot.to_string()),
    ]
}

#[test]
fn a_summon_takes_the_slot_the_game_gives_it_right_after() {
    let mut l = start();
    l.extend(minion(30, "SYN_OWN_A", 2, 1, (2, 2), &[]));
    l.extend(minion(31, "SYN_OWN_B", 2, 2, (2, 2), &[]));
    l.extend(minion(40, "SYN_FOE", 10, 1, (1, 9), &[]));
    l.push(block("TRIGGER", 31, "TAG_NOT_SET"));
    l.extend(shown_summon(50, "SYN_TOKEN", 2, 3));
    l.push(end());
    l.push(block("ATTACK", 30, ""));
    l.push(tag("GameEntity", "PROPOSED_DEFENDER", "40"));
    l.push(tag("40", "DAMAGE", "2"));
    l.push(end());
    let r = finished(l);
    let replay = replay(&r, 1);
    let token = &replay.minions[3];
    assert_eq!(token.card_id.as_deref(), Some("SYN_TOKEN"));
    assert_eq!(token.position, 3, "the slot set right after the summon");
}

#[test]
fn a_slot_set_after_the_next_step_is_the_board_shifting_not_the_summon() {
    let mut l = start();
    l.extend(minion(30, "SYN_OWN_A", 2, 1, (2, 2), &[]));
    l.extend(minion(31, "SYN_OWN_B", 2, 2, (2, 2), &[]));
    l.extend(minion(40, "SYN_FOE", 10, 1, (1, 9), &[]));
    l.push(block("TRIGGER", 31, "TAG_NOT_SET"));
    // Revealed in play with no slot, and no slot comes before the next step.
    let mut summon = shown_summon(50, "SYN_TOKEN", 2, 3);
    summon.pop();
    l.extend(summon);
    l.push(tag("30", "ZONE", "GRAVEYARD"));
    l.push(tag("50", "ZONE_POSITION", "2"));
    l.push(end());
    l.push(block("ATTACK", 31, ""));
    l.push(tag("GameEntity", "PROPOSED_DEFENDER", "40"));
    l.push(end());
    let r = finished(l);
    let replay = replay(&r, 1);
    assert_eq!(replay.minions[3].card_id.as_deref(), Some("SYN_TOKEN"));
    assert_eq!(
        replay.minions[3].position, 0,
        "a slot after a death is a shift: the summon keeps no slot"
    );
}

#[test]
fn duos_a_board_coming_in_keeps_each_minions_slot() {
    let mut l = start();
    for (i, id) in [61, 62, 63].into_iter().enumerate() {
        let tags = [
            ("CARDTYPE", "MINION".to_string()),
            ("CONTROLLER", "2".to_string()),
            ("ZONE", "SETASIDE".to_string()),
            ("ATK", "1".to_string()),
            ("HEALTH", "1".to_string()),
        ];
        l.extend(entity(id, &format!("SYN_IN_{i}"), &tags));
    }
    l.extend(minion(30, "SYN_MATE", 2, 1, (1, 1), &[]));
    l.extend(minion(40, "SYN_FOE_A", 10, 1, (5, 5), &[]));
    l.push(block("ATTACK", 30, ""));
    l.push(tag("GameEntity", "PROPOSED_DEFENDER", "40"));
    l.push(tag("30", "DAMAGE", "5"));
    l.push(end());
    l.push(tag("30", "ZONE", "GRAVEYARD"));
    l.extend(hero(103, "SYN_HERO_OWN", 2, 0, "PLAY"));
    l.push(tag("2", "HERO_ENTITY", "103"));
    // The board coming in was made before the combat and waits set aside
    // with no slot; each minion enters play and gets its slot one tag change
    // later, left to right.
    for (i, id) in [61, 62, 63].into_iter().enumerate() {
        l.push(tag(&id.to_string(), "ZONE", "PLAY"));
        l.push(tag(&id.to_string(), "ZONE_POSITION", &(i + 1).to_string()));
    }
    l.push(block("ATTACK", 61, ""));
    l.push(tag("GameEntity", "PROPOSED_DEFENDER", "40"));
    l.push(end());
    let r = finished(l);
    let replay = replay(&r, 1);
    let slots: Vec<_> = replay.minions[2..]
        .iter()
        .map(|m| (m.card_id.as_deref().unwrap(), m.position))
        .collect();
    assert_eq!(slots, [("SYN_IN_0", 1), ("SYN_IN_1", 2), ("SYN_IN_2", 3)]);
}

/// Each attack of the user's real games, played back the way the website
/// and the app draw a replay (a summon goes in at its slot, a minion that
/// died leaves at the next step), comes from the slot the game had the
/// attacker in, on both sides, in Solo and Duos.
#[test]
fn real_games_draw_every_attacker_in_its_own_slot() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/real");
    let mut checked = 0;
    for name in [
        "b253216_solo",
        "b253216_duos",
        "b253216_duos_gifts",
        "b253216_duos_rally",
    ] {
        let text = std::fs::read_to_string(dir.join(format!("{name}.log"))).expect("fixture");
        let reports = bg_parser::parse_reader(text.as_bytes()).expect("read");
        for round in reports.iter().flat_map(|r| &r.rounds) {
            let Some(replay) = &round.replay else {
                continue;
            };
            let mut boards: [Vec<usize>; 2] = [Vec::new(), Vec::new()];
            let index = |side: Side| usize::from(side == Side::Opponent);
            for (i, m) in replay.minions[..replay.start].iter().enumerate() {
                boards[index(m.side)].push(i);
            }
            let mut dying = Vec::new();
            let mut logged = round.attacks.iter();
            for e in &replay.events {
                for gone in dying.drain(..) {
                    boards.iter_mut().for_each(|b| b.retain(|&x| x != gone));
                }
                match e {
                    CombatEvent::Death { minion } => dying.push(*minion),
                    CombatEvent::Summon { minion } => {
                        let m = &replay.minions[*minion];
                        assert!(
                            m.position > 0,
                            "{name} round {}: a summon's slot",
                            round.number
                        );
                        let board = &mut boards[index(m.side)];
                        let at = usize::try_from(m.position - 1).unwrap().min(board.len());
                        board.insert(at, *minion);
                    }
                    CombatEvent::Attack { attacker, .. } => {
                        let side = replay.minions[*attacker].side;
                        let drawn = boards[index(side)].iter().position(|x| x == attacker);
                        let slot = logged.next().expect("the log's attack").attacker.position;
                        assert_eq!(
                            drawn.map(|d| d as i64 + 1),
                            Some(slot),
                            "{name} round {}: attack {checked}",
                            round.number
                        );
                        checked += 1;
                    }
                    _ => {}
                }
            }
        }
    }
    assert!(checked > 100, "{checked} attacks checked");
}

// ---------- Parser revision 13: beats, damage sources, keyword marks ----------

fn nested_end(depth: usize) -> String {
    line(depth, "BLOCK_END")
}

#[test]
fn each_beat_is_the_run_of_steps_one_log_block_gives() {
    let r = finished(won_combat());
    let replay = replay(&r, 1);
    // Start-of-combat trigger and its damage; the first attack with its hit
    // and shield; a death outside any block; the second attack; a death; the
    // deathrattle and its summon, with the token's buff after it (outside any
    // block: the game refreshing it); the token's attack; a death; the hero's
    // hit.
    assert_eq!(replay.beats, [2, 3, 1, 3, 1, 3, 3, 1, 1]);
    assert_eq!(replay.beats.iter().sum::<usize>(), replay.events.len());
}

#[test]
fn a_board_wide_buff_and_simultaneous_deaths_are_one_beat_each() {
    let mut l = start();
    l.extend(minion(30, "SYN_BUFFER", 2, 1, (1, 1), &[]));
    l.extend(minion(31, "SYN_A", 2, 2, (1, 1), &[]));
    l.extend(minion(32, "SYN_B", 2, 3, (1, 1), &[]));
    l.extend(minion(40, "SYN_FOE_A", 10, 1, (1, 1), &[]));
    l.extend(minion(41, "SYN_FOE_B", 10, 2, (1, 1), &[]));
    l.push(block("TRIGGER", 30, "TAG_NOT_SET"));
    for id in ["30", "31", "32"] {
        l.push(tag(id, "ATK", "3"));
        l.push(tag(id, "HEALTH", "3"));
    }
    l.push(end());
    l.push(block("ATTACK", 30, ""));
    l.push(tag("GameEntity", "PROPOSED_DEFENDER", "40"));
    l.push(tag("40", "DAMAGE", "3"));
    l.push(tag("30", "DAMAGE", "1"));
    l.push(end());
    l.push(line(0, "BLOCK_START BlockType=DEATHS Entity=GameEntity EffectCardId=System.Collections.Generic.List`1[System.String] EffectIndex=0 Target=0 SubOption=-1 "));
    l.push(tag("40", "ZONE", "GRAVEYARD"));
    l.push(tag("41", "ZONE", "GRAVEYARD"));
    l.push(end());
    let r = finished(l);
    let replay = replay(&r, 1);
    assert_eq!(replay.beats, [4, 3, 2], "{:?}", replay.events);
    assert!(matches!(
        replay.events[1..4],
        [
            CombatEvent::Stats {
                minion: 0,
                atk: 3,
                health: 3
            },
            CombatEvent::Stats {
                minion: 1,
                atk: 3,
                health: 3
            },
            CombatEvent::Stats {
                minion: 2,
                atk: 3,
                health: 3
            },
        ]
    ));
}

#[test]
fn each_damage_names_where_it_came_from() {
    let r = finished(won_combat());
    let replay = replay(&r, 1);
    let sources: Vec<_> = replay
        .events
        .iter()
        .filter_map(|e| match e {
            CombatEvent::Damage { minion, source, .. } => Some((*minion, source.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(
        sources,
        [
            (3, by(Cause::StartOfCombat, 1)),
            (2, by(Cause::Attack, 0)),
            // The attacker is hurt by its defender.
            (1, by(Cause::Attack, 3)),
            (3, by(Cause::Attack, 1)),
            (3, by(Cause::Attack, 4)),
            (4, by(Cause::Attack, 3)),
        ]
    );
}

#[test]
fn a_hero_power_or_an_unknown_block_says_so() {
    let mut l = start();
    l.extend(minion(30, "SYN_OWN", 2, 1, (1, 5), &[]));
    l.extend(minion(40, "SYN_FOE", 10, 1, (1, 5), &[]));
    l.extend(entity(
        45,
        "SYN_FOE_POWER",
        &[
            ("CARDTYPE", "HERO_POWER".to_string()),
            ("CONTROLLER", "10".to_string()),
            ("ZONE", "PLAY".to_string()),
        ],
    ));
    l.push(block("TRIGGER", 45, "TAG_NOT_SET"));
    l.push(tag("30", "DAMAGE", "2"));
    l.push(end());
    // Damage outside any block the log names: unknown.
    l.push(tag("30", "DAMAGE", "3"));
    l.push(block("ATTACK", 30, ""));
    l.push(tag("GameEntity", "PROPOSED_DEFENDER", "40"));
    l.push(end());
    let r = finished(l);
    let replay = replay(&r, 1);
    let sources: Vec<_> = replay
        .events
        .iter()
        .filter_map(|e| match e {
            CombatEvent::Damage { source, .. } => Some(source.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        sources,
        [
            Some(DamageSource {
                how: Cause::HeroPower,
                minion: None,
                card_id: Some("SYN_FOE_POWER".into()),
                side: Some(Side::Opponent),
            }),
            None,
        ]
    );
    let json = serde_json::to_value(&replay.events).unwrap();
    let damages: Vec<_> = json
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["kind"] == "damage")
        .collect();
    assert_eq!(
        damages[0]["source"],
        serde_json::json!({"how": "hero_power", "card_id": "SYN_FOE_POWER", "side": "opponent"})
    );
    assert_eq!(
        damages[1]["source"],
        serde_json::Value::Null,
        "unknown is written, never left out"
    );
}

/// A Rally that gives Venomous as its minion attacks, nested in the attack
/// as the game logs it: the attack, then the Rally with the mark appearing,
/// then the hit, which uses the Venomous up.
#[test]
fn a_keyword_given_or_used_mid_combat_is_a_step_of_its_own_block() {
    let mut l = start();
    l.extend(minion(30, "SYN_RALLY", 2, 1, (3, 3), &[]));
    l.extend(minion(31, "SYN_MATE", 2, 2, (1, 1), &[]));
    l.extend(minion(40, "SYN_FOE", 10, 1, (2, 9), &["TAUNT"]));
    l.push(block("ATTACK", 30, ""));
    l.push(tag("GameEntity", "PROPOSED_DEFENDER", "40"));
    l.push(nested_block(1, "TRIGGER", 30, "BACON_RALLY"));
    l.push(line(2, "TAG_CHANGE Entity=30 tag=VENOMOUS value=1 "));
    l.push(nested_end(1));
    l.push(tag("40", "DAMAGE", "3"));
    l.push(tag("30", "VENOMOUS", "0"));
    l.push(tag("30", "DAMAGE", "2"));
    l.push(end());
    l.push(tag("40", "TAUNT", "0"));
    let r = finished(l);
    let replay = replay(&r, 1);
    use CombatEvent::*;
    let kinds: Vec<_> = replay
        .events
        .iter()
        .map(|e| match e {
            Attack { .. } => "attack".to_string(),
            Trigger { trigger, .. } => format!("{trigger:?}"),
            Keyword {
                minion,
                keyword,
                on,
            } => format!("{keyword}:{minion}:{on}"),
            Damage { minion, .. } => format!("damage:{minion}"),
            other => format!("{other:?}"),
        })
        .collect();
    assert_eq!(
        kinds,
        [
            "attack",
            "Rally",
            "venomous:0:true",
            "damage:2",
            "venomous:0:false",
            "damage:0",
            "taunt:2:false"
        ]
    );
    assert_eq!(
        replay.beats,
        [1, 2, 4],
        "the attack, the Rally, the hit with the Taunt the game takes off after it"
    );
}

/// The owner's own game: as a Rally murloc attacks, its Rally gives
/// Venomous to friendly minions in a beat of its own, between the attack
/// and its hit, as the game plays it.
#[test]
fn a_real_rally_gives_venomous_in_a_beat_of_its_own() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/real");
    let text = std::fs::read_to_string(dir.join("b253216_duos_rally.log")).expect("fixture");
    let reports = bg_parser::parse_reader(text.as_bytes()).expect("read");
    let mut found = 0;
    for round in &reports[0].rounds {
        let Some(replay) = &round.replay else {
            continue;
        };
        assert_eq!(replay.beats.iter().sum::<usize>(), replay.events.len());
        let mut at = 0;
        for &beat in &replay.beats {
            let steps = &replay.events[at..at + beat];
            at += beat;
            let rally = matches!(
                steps.first(),
                Some(CombatEvent::Trigger {
                    trigger: TriggerKind::Rally,
                    ..
                })
            );
            let given: Vec<usize> = steps
                .iter()
                .filter_map(|e| match e {
                    CombatEvent::Keyword {
                        minion,
                        keyword: "venomous",
                        on: true,
                    } => Some(*minion),
                    _ => None,
                })
                .collect();
            if !rally || given.is_empty() {
                continue;
            }
            found += 1;
            // The attack is the beat just before, and its hit comes after.
            let before = at - beat - 1;
            assert!(
                matches!(replay.events[before], CombatEvent::Attack { .. }),
                "round {}",
                round.number
            );
            assert!(given.iter().all(|&m| replay.minions[m].card_id.is_some()));
        }
    }
    assert!(found >= 2, "{found} Rally beats giving Venomous");
}

/// A log that lost a block's end, an end deeper than anything open, and
/// blocks nested past the tracked depth still give beats that add up to the
/// events, each later block its own.
#[test]
fn the_block_stack_survives_lost_ends_and_deep_nesting() {
    let mut l = start();
    l.extend(minion(30, "SYN_OWN", 2, 1, (2, 9), &[]));
    l.extend(minion(40, "SYN_FOE", 10, 1, (1, 9), &[]));
    // A trigger whose end the log lost: the attack after it, at the same
    // depth, closes it.
    l.push(block("TRIGGER", 30, "TAG_NOT_SET"));
    l.push(tag("30", "ATK", "3"));
    l.push(block("ATTACK", 30, ""));
    l.push(tag("GameEntity", "PROPOSED_DEFENDER", "40"));
    l.push(tag("40", "DAMAGE", "3"));
    // An end deeper than anything open closes nothing.
    l.push(line(3, "BLOCK_END"));
    l.push(tag("30", "DAMAGE", "1"));
    l.push(end());
    // Seventy blocks nested in each other, then a hit at the deepest.
    for depth in 0..70 {
        l.push(nested_block(depth, "TRIGGER", 30, "TAG_NOT_SET"));
    }
    l.push(line(70, "TAG_CHANGE Entity=40 tag=DAMAGE value=4 "));
    for depth in (0..70).rev() {
        l.push(line(depth, "BLOCK_END"));
    }
    l.push(block("ATTACK", 40, ""));
    l.push(tag("GameEntity", "PROPOSED_DEFENDER", "30"));
    l.push(tag("30", "DAMAGE", "2"));
    l.push(end());
    let r = finished(l);
    let replay = replay(&r, 1);
    assert_eq!(replay.beats.iter().sum::<usize>(), replay.events.len());
    let kinds: Vec<_> = replay
        .events
        .iter()
        .map(|e| match e {
            CombatEvent::Attack { .. } => "attack",
            CombatEvent::Damage { .. } => "damage",
            CombatEvent::Stats { .. } => "stats",
            CombatEvent::Trigger { .. } => "trigger",
            _ => "other",
        })
        .collect();
    assert_eq!(
        kinds,
        ["trigger", "stats", "attack", "damage", "damage", "damage", "attack", "damage"]
    );
    assert_eq!(replay.beats, [2, 3, 1, 2]);
}
