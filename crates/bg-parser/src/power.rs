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
    Power(&'a str),
    Game(&'a str),
    Other,
}

const POWER: &str = "GameState.DebugPrintPower() - ";
const GAME: &str = "GameState.DebugPrintGame() - ";
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
        return Line::Power(line[pos + POWER.len()..].trim());
    }
    if let Some(pos) = line.find(GAME) {
        return Line::Game(line[pos + GAME.len()..].trim());
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
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_lines() {
        assert_eq!(
            classify("D 12:00:00.0000000 GameState.DebugPrintPower() -     CREATE_GAME\r\n"),
            Line::Power("CREATE_GAME")
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
    fn unknown_opcode_is_an_error_not_skipped() {
        assert_eq!(
            parse_power("NEW_THING x=1"),
            Err(ParseError("NotImplementedError"))
        );
    }
}
