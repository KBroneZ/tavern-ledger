//! Tokenizes `GameState.DebugPrintPower()` lines into packets.
//!
//! Written from the format documented in `docs/research/parser-hslog.md` and
//! the line grammar of hslog (MIT), which the Python prototype used.

/// A tag value: the log prints numbers or enum names (ZONE=PLAY).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Int(i64),
    Name(String),
}

impl Value {
    fn parse(raw: &str) -> Value {
        raw.parse::<i64>()
            .map(Value::Int)
            .unwrap_or_else(|_| Value::Name(raw.to_string()))
    }
}

/// How a packet names an entity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntityRef {
    Id(i64),
    Game,
    /// A player name. Never stored or printed outside the parser.
    Name(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Packet {
    CreateGame,
    GameEntity {
        id: i64,
    },
    Player {
        id: i64,
        player_id: i64,
        lo: u64,
    },
    FullEntity {
        id: i64,
        card_id: Option<String>,
    },
    ShowEntity {
        entity: EntityRef,
        card_id: String,
    },
    ChangeEntity {
        entity: EntityRef,
        card_id: String,
    },
    HideEntity {
        entity: EntityRef,
    },
    Tag {
        tag: String,
        value: Value,
    },
    TagChange {
        entity: EntityRef,
        tag: String,
        value: Value,
    },
    BlockStart {
        block_type: String,
        entity: EntityRef,
        /// The block's target; None when the log gives `Target=0`.
        target: Option<EntityRef>,
    },
    BlockEnd,
    /// Entity references in META_DATA / SUB_SPELL detail lines. Only names
    /// matter: hslog registers them, which can change how names resolve.
    Mention(EntityRef),
    /// Lines that carry nothing the report needs.
    Ignored,
}

/// Which part of the log a line belongs to.
#[derive(Debug, PartialEq, Eq)]
pub enum Line<'a> {
    /// A power packet and how deep it is nested: the log indents each
    /// level of BLOCK_START by four spaces, so 0 is a top-level packet.
    Power(&'a str, usize),
    Game(&'a str),
    /// The client sending one of the player's options (T-205).
    SendOption,
    /// The client sending a choice; the text is its type (GENERAL, MULLIGAN...).
    SendChoice(&'a str),
    Other,
}

const POWER: &str = "GameState.DebugPrintPower() - ";
const GAME: &str = "GameState.DebugPrintGame() - ";
const SEND_OPTION: &str = "GameState.SendOption() - selectedOption=";
const SEND_CHOICES: &str = "GameState.SendChoices() - id=";
const IGNORED_OPCODES: &[&str] = &[
    "META_DATA",
    "SUB_SPELL_START",
    "SUB_SPELL_END",
    "CACHED_TAG_FOR_DORMANT_CHANGE",
    "VO_SPELL",
    "SHUFFLE_DECK",
    "RESET_GAME",
    "ERROR:",
];

/// Splits "D 12:00:00.0000000 GameState.DebugPrintPower() - body".
pub fn classify(line: &str) -> Line<'_> {
    let line = line.trim_end_matches(['\r', '\n']);
    if let Some(pos) = line.find(POWER) {
        let body = &line[pos + POWER.len()..];
        let indent = body.len() - body.trim_start().len();
        return Line::Power(body.trim(), indent);
    }
    if let Some(pos) = line.find(GAME) {
        return Line::Game(line[pos + GAME.len()..].trim());
    }
    if line.contains(SEND_OPTION) {
        return Line::SendOption;
    }
    if let Some(pos) = line.find(SEND_CHOICES) {
        // "id=N ChoiceType=GENERAL"; the chosen entities follow on their own lines.
        let rest = &line[pos + SEND_CHOICES.len()..];
        if let Some((_, kind)) = rest.split_once(" ChoiceType=") {
            return Line::SendChoice(kind.trim());
        }
    }
    Line::Other
}

/// Parse error: the name mirrors the Python prototype's exception names, so
/// both report the same problem. It never contains log text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParseError(pub &'static str);

pub fn parse_power(body: &str) -> Result<Packet, ParseError> {
    let opcode = body.split_whitespace().next().unwrap_or("");
    let bad = ParseError("RegexParsingError");
    Ok(match opcode {
        "CREATE_GAME" => Packet::CreateGame,
        "GameEntity" => Packet::GameEntity {
            id: int_after(body, "EntityID=").ok_or(bad)?,
        },
        "Player" => parse_player(body).ok_or(bad)?,
        "FULL_ENTITY" => parse_full_entity(body).ok_or(bad)?,
        "SHOW_ENTITY" => {
            let (entity, card_id) = parse_updating(body).ok_or(bad)?;
            Packet::ShowEntity { entity, card_id }
        }
        "CHANGE_ENTITY" => {
            let (entity, card_id) = parse_updating(body).ok_or(bad)?;
            Packet::ChangeEntity { entity, card_id }
        }
        "HIDE_ENTITY" => {
            let rest = body.strip_prefix("HIDE_ENTITY - Entity=").ok_or(bad)?;
            let end = rest.rfind(" tag=").ok_or(bad)?;
            Packet::HideEntity {
                entity: parse_entity(&rest[..end]),
            }
        }
        "TAG_CHANGE" => parse_tag_change(body).ok_or(bad)?,
        "BLOCK_START" => parse_block_start(body).ok_or(bad)?,
        "BLOCK_END" => Packet::BlockEnd,
        _ if opcode.starts_with("tag=") => {
            let (tag, value) = parse_tag_value(body).ok_or(bad)?;
            Packet::Tag { tag, value }
        }
        _ if opcode.starts_with("Info[")
            || opcode.starts_with("Targets[")
            || opcode == "Source" =>
        {
            let (_, entity) = body.split_once(" = ").ok_or(bad)?;
            Packet::Mention(parse_entity(entity.trim()))
        }
        _ if IGNORED_OPCODES.contains(&opcode) => Packet::Ignored,
        _ => return Err(ParseError("NotImplementedError")),
    })
}

/// Parses an entity as hslog does: GameEntity, a number, "[... id=N ...]" or a name.
pub fn parse_entity(raw: &str) -> EntityRef {
    if raw == "GameEntity" {
        return EntityRef::Game;
    }
    if !raw.is_empty() && raw.bytes().all(|b| b.is_ascii_digit()) {
        if let Ok(id) = raw.parse() {
            return EntityRef::Id(id);
        }
    }
    if raw.starts_with('[') && raw.ends_with(']') {
        // The last "id=N" wins, like hslog's greedy regex.
        for (pos, _) in raw.rmatch_indices("id=") {
            let digits: String = raw[pos + 3..]
                .chars()
                .take_while(char::is_ascii_digit)
                .collect();
            if let Ok(id) = digits.parse() {
                return EntityRef::Id(id);
            }
        }
    }
    EntityRef::Name(raw.to_string())
}

const NAME_KEY: &str = "[entityName=";
const UNKNOWN_ENTITY: &str = "UNKNOWN ENTITY";
/// How far past "[entityName=" the " id=N zone=" may be (longer names are
/// not card names), and how far past it the closing "]" may be. Bounding
/// both keeps a long line of broken references linear.
const NAME_WINDOW: usize = 512;
const BRACKET_WINDOW: usize = 256;
const MAX_CARD_ID: usize = 64;

/// Card names the log prints inside "[entityName=NAME id=N zone=... cardId=ID ...]",
/// as (card id, name). These are in the game client's language. The card id
/// is only read inside the same brackets. Entities without a card id
/// (players, whose name is the BattleTag), names that look like a BattleTag
/// and hidden cards ("UNKNOWN ENTITY") give nothing.
pub fn card_names(body: &str) -> Vec<(&str, &str)> {
    let mut out = Vec::new();
    let mut rest = body;
    while let Some(start) = rest.find(NAME_KEY) {
        rest = &rest[start + NAME_KEY.len()..];
        let Some(name_end) = name_end(rest) else {
            continue;
        };
        let name = &rest[..name_end];
        rest = &rest[name_end..];
        let Some(close) = rest.bytes().take(BRACKET_WINDOW).position(|b| b == b']') else {
            continue;
        };
        let card = rest[..close]
            .split(' ')
            .find_map(|field| field.strip_prefix("cardId="))
            .unwrap_or("");
        if is_card_id(card) && is_card_name(name) {
            out.push((card, name));
        }
        rest = &rest[close..];
    }
    out
}

/// Where the name ends: the first " id=<digits> zone=", as hslog's lazy regex.
fn name_end(text: &str) -> Option<usize> {
    text.match_indices(" id=")
        .map(|(pos, _)| pos)
        .take_while(|&pos| pos <= NAME_WINDOW)
        .find(|&pos| {
            let tail = &text[pos + " id=".len()..];
            let digits = tail.bytes().take_while(u8::is_ascii_digit).count();
            digits > 0 && tail[digits..].starts_with(" zone=")
        })
}

fn is_card_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_CARD_ID
        && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// "Name#1234" is a BattleTag, never a card name.
fn is_card_name(name: &str) -> bool {
    let looks_like_battletag = name
        .rsplit_once('#')
        .is_some_and(|(_, tail)| !tail.is_empty() && tail.bytes().all(|b| b.is_ascii_digit()));
    !name.is_empty() && !name.starts_with(UNKNOWN_ENTITY) && !looks_like_battletag
}

fn int_after(body: &str, key: &str) -> Option<i64> {
    let start = body.find(key)? + key.len();
    let digits: String = body[start..]
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '-')
        .collect();
    digits.parse().ok()
}

fn parse_player(body: &str) -> Option<Packet> {
    let id = int_after(body, "EntityID=")?;
    let player_id = int_after(body, "PlayerID=")?;
    let lo = int_after(body, " lo=")?;
    Some(Packet::Player {
        id,
        player_id,
        lo: lo.try_into().ok()?,
    })
}

fn parse_full_entity(body: &str) -> Option<Packet> {
    let rest = body.strip_prefix("FULL_ENTITY - Creating ID=")?;
    let (id, card) = rest.split_once(" CardID=")?;
    let card = card.trim();
    Some(Packet::FullEntity {
        id: id.parse().ok()?,
        card_id: (!card.is_empty()).then(|| card.to_string()),
    })
}

/// "SHOW_ENTITY - Updating Entity=<e> CardID=<id>" (same shape for CHANGE_ENTITY).
fn parse_updating(body: &str) -> Option<(EntityRef, String)> {
    let start = body.find("Updating Entity=")? + "Updating Entity=".len();
    let rest = &body[start..];
    let end = rest.rfind(" CardID=")?;
    let card = rest[end + " CardID=".len()..].trim();
    if card.is_empty() {
        return None;
    }
    Some((parse_entity(&rest[..end]), card.to_string()))
}

fn parse_tag_value(text: &str) -> Option<(String, Value)> {
    let rest = text.strip_prefix("tag=")?;
    let (tag, rest) = rest.split_once(" value=")?;
    let value = rest.split_whitespace().next()?;
    Some((tag.to_string(), Value::parse(value)))
}

fn parse_tag_change(body: &str) -> Option<Packet> {
    let rest = body.strip_prefix("TAG_CHANGE Entity=")?;
    let split = rest.rfind(" tag=")?;
    let (tag, value) = parse_tag_value(&rest[split + 1..])?;
    Some(Packet::TagChange {
        entity: parse_entity(&rest[..split]),
        tag,
        value,
    })
}

fn parse_block_start(body: &str) -> Option<Packet> {
    let rest = body.strip_prefix("BLOCK_START BlockType=")?;
    let (block_type, rest) = rest.split_once(" Entity=")?;
    let end = rest.find(" EffectCardId=")?;
    Some(Packet::BlockStart {
        block_type: block_type.to_string(),
        entity: parse_entity(&rest[..end]),
        target: block_target(&rest[end..]),
    })
}

/// "... Target=<entity> SubOption=..." -> the entity; None for `Target=0`.
fn block_target(rest: &str) -> Option<EntityRef> {
    let start = rest.find(" Target=")? + " Target=".len();
    let rest = &rest[start..];
    let end = rest.rfind(" SubOption=").unwrap_or(rest.len());
    match parse_entity(rest[..end].trim()) {
        EntityRef::Id(0) => None,
        target => Some(target),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_lines() {
        assert_eq!(
            classify("D 12:00:00.0000000 GameState.DebugPrintPower() -     CREATE_GAME\r\n"),
            Line::Power("CREATE_GAME", 4)
        );
        assert_eq!(
            classify("D 1 GameState.DebugPrintPower() - BLOCK_END"),
            Line::Power("BLOCK_END", 0)
        );
        assert_eq!(
            classify(
                "D 1 GameState.SendOption() - selectedOption=7 selectedSubOption=-1 \
                 selectedTarget=336 selectedPosition=0"
            ),
            Line::SendOption
        );
        assert_eq!(
            classify("D 1 GameState.SendChoices() - id=2 ChoiceType=GENERAL"),
            Line::SendChoice("GENERAL")
        );
        assert_eq!(
            classify("D 1 GameState.SendChoices() -   m_chosenEntities[0]=4"),
            Line::Other
        );
        assert_eq!(
            classify("D 1 GameState.DebugPrintGame() - GameType=GT_BATTLEGROUNDS"),
            Line::Game("GameType=GT_BATTLEGROUNDS")
        );
        assert_eq!(
            classify("D 1 PowerTaskList.DebugPrintPower() - CREATE_GAME"),
            Line::Other
        );
    }

    #[test]
    fn names_cards_from_bracketed_entities() {
        let body = "TAG_CHANGE Entity=[entityName=Lord Jaraxxus id=10 zone=PLAY zonePos=0 \
            cardId=TB_BaconShop_HERO_37 player=2] tag=ZONE value=PLAY";
        assert_eq!(
            card_names(body),
            vec![("TB_BaconShop_HERO_37", "Lord Jaraxxus")]
        );
        // hslog's name match is lazy up to " id=N zone=": a name may hold " id=".
        let body = "Info[0] = [entityName=Some id=7 Minion id=103 zone=HAND zonePos=1 \
            cardId=X player=7]";
        assert_eq!(card_names(body), vec![("X", "Some id=7 Minion")]);
    }

    #[test]
    fn players_and_unknown_entities_have_no_card_name() {
        assert!(card_names(
            "TAG_CHANGE Entity=[entityName=Someone#1234 id=2 zone=PLAY zonePos=0 cardId= player=2] tag=A value=1"
        )
        .is_empty());
        assert!(card_names(
            "TAG_CHANGE Entity=[entityName=UNKNOWN ENTITY [cardType=INVALID] id=9 zone=HAND zonePos=0 cardId=X player=2] tag=A value=1"
        )
        .is_empty());
        assert!(card_names("TAG_CHANGE Entity=Someone#1234 tag=A value=1").is_empty());
        assert!(card_names("TAG_CHANGE Entity=[entityName=Cut").is_empty());
    }

    #[test]
    fn a_card_id_is_never_taken_from_another_entity() {
        // A player bracket without cardId, then a hero on the same line.
        let body = "BLOCK_START BlockType=ATTACK Entity=[entityName=Someone#1234 id=2 zone=PLAY \
            zonePos=0 player=2] Target=[entityName=Hero id=10 zone=PLAY zonePos=0 \
            cardId=TB_BaconShop_HERO_37 player=2]";
        assert_eq!(card_names(body), vec![("TB_BaconShop_HERO_37", "Hero")]);
    }

    #[test]
    fn battletag_like_names_and_odd_card_ids_are_dropped() {
        assert!(card_names(
            "Info[0] = [entityName=Someone#1234 id=2 zone=PLAY zonePos=0 cardId=TB_BaconShop_HERO_1 player=2]"
        )
        .is_empty());
        assert!(card_names(
            "Info[0] = [entityName=Hero id=2 zone=PLAY zonePos=0 cardId=BAD<id> player=2]"
        )
        .is_empty());
        let long = format!(
            "Info[0] = [entityName=Hero id=2 zone=PLAY zonePos=0 cardId={} player=2]",
            "A".repeat(65)
        );
        assert!(card_names(&long).is_empty());
    }

    #[test]
    fn a_huge_line_of_broken_references_is_linear() {
        let line = "[entityName=A id=1 zone=X ".repeat(40_000);
        let start = std::time::Instant::now();
        assert!(card_names(&line).is_empty());
        assert!(start.elapsed() < std::time::Duration::from_secs(1));
    }

    #[test]
    fn parses_entity_references() {
        assert_eq!(parse_entity("GameEntity"), EntityRef::Game);
        assert_eq!(parse_entity("42"), EntityRef::Id(42));
        assert_eq!(
            parse_entity("[entityName=Some id=7 Minion id=103 zone=HAND cardId=X player=7]"),
            EntityRef::Id(103)
        );
        assert_eq!(
            parse_entity("Someone#1234"),
            EntityRef::Name("Someone#1234".into())
        );
    }

    #[test]
    fn parses_tag_change_with_bracketed_entity_and_def_change() {
        let p = parse_power(
            "TAG_CHANGE Entity=[entityName=A id=5 zone=PLAY] tag=ZONE value=GRAVEYARD DEF CHANGE",
        )
        .unwrap();
        assert_eq!(
            p,
            Packet::TagChange {
                entity: EntityRef::Id(5),
                tag: "ZONE".into(),
                value: Value::Name("GRAVEYARD".into())
            }
        );
    }

    #[test]
    fn parses_entities_and_players() {
        assert_eq!(
            parse_power("FULL_ENTITY - Creating ID=37 CardID=").unwrap(),
            Packet::FullEntity {
                id: 37,
                card_id: None
            }
        );
        assert_eq!(
            parse_power("Player EntityID=3 PlayerID=10 GameAccountId=[hi=0 lo=0]").unwrap(),
            Packet::Player {
                id: 3,
                player_id: 10,
                lo: 0
            }
        );
        assert_eq!(
            parse_power("SHOW_ENTITY - Updating Entity=249 CardID=TB_X").unwrap(),
            Packet::ShowEntity {
                entity: EntityRef::Id(249),
                card_id: "TB_X".into()
            }
        );
    }

    #[test]
    fn block_start_keeps_its_target() {
        let p = parse_power(
            "BLOCK_START BlockType=PLAY Entity=[entityName=Buy id=337 zone=PLAY zonePos=0 \
             cardId=TB_BaconShop_DragBuy player=7] EffectCardId=System.Collections.Generic.\
             List`1[System.String] EffectIndex=0 Target=[entityName=Some Minion id=336 \
             zone=PLAY zonePos=1 cardId=BG36_345 player=15] SubOption=-1 ",
        )
        .unwrap();
        assert_eq!(
            p,
            Packet::BlockStart {
                block_type: "PLAY".into(),
                entity: EntityRef::Id(337),
                target: Some(EntityRef::Id(336)),
            }
        );
        let p = parse_power(
            "BLOCK_START BlockType=PLAY Entity=1023 EffectCardId=X EffectIndex=0 Target=0 SubOption=-1",
        )
        .unwrap();
        assert!(matches!(p, Packet::BlockStart { target: None, .. }));
        // Scrubbed fixtures keep only the id.
        let p = parse_power(
            "BLOCK_START BlockType=PLAY Entity=337 EffectCardId=X EffectIndex=0 Target=336 SubOption=-1",
        )
        .unwrap();
        assert!(matches!(
            p,
            Packet::BlockStart {
                target: Some(EntityRef::Id(336)),
                ..
            }
        ));
    }

    #[test]
    fn unknown_opcode_is_an_error_not_skipped() {
        assert_eq!(
            parse_power("NEW_THING x=1"),
            Err(ParseError("NotImplementedError"))
        );
    }
}
