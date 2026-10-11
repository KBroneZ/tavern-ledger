//! Reads the page's request: both teams' boards with the card facts the
//! page took from its card data. The request is untrusted input: every
//! field is checked, a wrong one refuses the whole request with a short
//! reason, and nothing here panics.

use combat_sim::{BaseCard, Keywords, Minion, Team, Tribes, Warband};
use serde_json::{Map, Value};

/// Far above any real request (four boards of seven minions).
pub const MAX_INPUT_BYTES: usize = 64 * 1024;
/// A team brings one board in Solo, two in Duos.
const MAX_BOARDS: usize = 2;
/// The engine takes at most 7; a few more still get its own refusal.
const MAX_MINIONS: usize = 16;
const MAX_TIER: i64 = 7;
const MAX_NAMES: usize = 16;
const MAX_NAME: usize = 32;
/// The game's cap on the damage to a hero.
const MAX_CAP: i64 = 1_000;

/// Card facts a board does not show, as the page sends them (the same
/// words as `crates/cards` keeps: types `BEAST`, keywords `DIVINE_SHIELD`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Facts {
    pub tier: Option<u8>,
    pub tribes: Tribes,
    pub base: BaseCard,
}

/// One request: the player's team, the opponent's team and the damage cap.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub player: Team,
    pub opponent: Team,
    pub damage_cap: Option<i64>,
    /// Card ids with no facts: the odds need them (as in the app).
    pub missing: Vec<String>,
}

type Res<T> = Result<T, String>;

fn object<'a>(v: &'a Value, what: &str) -> Res<&'a Map<String, Value>> {
    v.as_object()
        .ok_or_else(|| format!("{what} is not an object"))
}

fn int(v: Option<&Value>, what: &str, min: i64, max: i64) -> Res<i64> {
    v.and_then(Value::as_i64)
        .filter(|n| (min..=max).contains(n))
        .ok_or_else(|| format!("{what} is not a whole number from {min} to {max}"))
}

/// `null` or missing is unknown; anything else must be a tier.
fn tier(v: Option<&Value>, what: &str) -> Res<Option<u8>> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(_) => Ok(Some(int(v, what, 1, MAX_TIER)? as u8)),
    }
}

fn names<'a>(v: Option<&'a Value>, what: &str) -> Res<Vec<&'a str>> {
    let Some(v) = v else { return Ok(Vec::new()) };
    let list = v
        .as_array()
        .filter(|l| l.len() <= MAX_NAMES)
        .ok_or_else(|| format!("{what} is not a short list"))?;
    list.iter()
        .map(|n| {
            n.as_str()
                .filter(|s| !s.is_empty() && s.len() <= MAX_NAME)
                .filter(|s| s.bytes().all(|b| b.is_ascii_alphabetic() || b == b'_'))
                .ok_or_else(|| format!("{what} holds a bad name"))
        })
        .collect()
}

/// A card id as the parser and the card data write it.
pub fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// Keywords on the board: every name must be one the page may send. Names
/// the engine does not track (deathrattle, rally) are allowed and dropped,
/// as the app does.
fn board_keywords(v: Option<&Value>) -> Res<Keywords> {
    const SHOWN_ONLY: [&str; 2] = ["deathrattle", "rally"];
    let mut out = Keywords::NONE;
    for name in names(v, "keywords")? {
        match Keywords::from_name(name) {
            Some(k) => out = out.union(k),
            None if SHOWN_ONLY.contains(&name) => {}
            None => return Err(format!("unknown keyword {name}")),
        }
    }
    Ok(out)
}

/// The card data's facts. Types and keywords the engine does not know are
/// left out, exactly as `tracker::odds::Facts::from_names` does.
fn facts(v: &Value) -> Res<Option<Facts>> {
    if v.is_null() {
        return Ok(None);
    }
    let f = object(v, "facts")?;
    let tribes = names(f.get("tribes"), "tribes")?
        .into_iter()
        .filter_map(Tribes::from_name)
        .fold(Tribes::NONE, Tribes::union);
    let keywords = names(f.get("keywords"), "card keywords")?
        .into_iter()
        .filter_map(Keywords::from_name)
        .fold(Keywords::NONE, Keywords::union);
    Ok(Some(Facts {
        tier: tier(f.get("tier"), "card tier")?,
        tribes,
        base: BaseCard {
            attack: int(f.get("attack"), "card attack", 0, 1_000)? as i32,
            health: int(f.get("health"), "card health", 1, 1_000)? as i32,
            keywords,
        },
    }))
}

fn minion(v: &Value, missing: &mut Vec<String>) -> Res<Option<Minion>> {
    let m = object(v, "minion")?;
    let id = m
        .get("card_id")
        .and_then(Value::as_str)
        .filter(|id| valid_id(id))
        .ok_or("a minion has no valid card id")?;
    let golden = match m.get("golden") {
        None => false,
        Some(g) => g.as_bool().ok_or("golden is not true or false")?,
    };
    let attack = int(m.get("attack"), "attack", 0, i64::from(i32::MAX))? as i32;
    let health = int(m.get("health"), "health", 1, i64::from(i32::MAX))? as i32;
    let keywords = board_keywords(m.get("keywords"))?;
    let Some(f) = facts(m.get("facts").unwrap_or(&Value::Null))? else {
        if !missing.iter().any(|x| x == id) {
            missing.push(id.to_string());
        }
        return Ok(None);
    };
    Ok(Some(Minion {
        card_id: id.to_string(),
        golden,
        attack,
        health,
        keywords,
        tribes: f.tribes,
        tier: f.tier,
        base: Some(f.base),
    }))
}

fn board(v: &Value, missing: &mut Vec<String>) -> Res<Warband> {
    let b = object(v, "board")?;
    let list = b
        .get("minions")
        .and_then(Value::as_array)
        .filter(|l| l.len() <= MAX_MINIONS)
        .ok_or("a board has no minion list, or a far too long one")?;
    let mut minions = Vec::new();
    for m in list {
        minions.extend(minion(m, missing)?);
    }
    Ok(Warband {
        minions,
        tavern_tier: tier(b.get("tier"), "tavern tier")?,
    })
}

fn team(v: Option<&Value>, who: &str, missing: &mut Vec<String>) -> Res<Team> {
    let boards = v
        .and_then(Value::as_array)
        .filter(|l| (1..=MAX_BOARDS).contains(&l.len()))
        .ok_or_else(|| format!("{who} needs one or two boards"))?;
    let first = board(&boards[0], missing)?;
    Ok(match boards.get(1) {
        Some(second) => Team::duo(first, board(second, missing)?),
        None => Team::solo(first),
    })
}

/// The request in the engine's form, or why it is refused.
pub fn read(text: &str) -> Res<Request> {
    if text.len() > MAX_INPUT_BYTES {
        return Err("the request is too large".into());
    }
    let v: Value = serde_json::from_str(text).map_err(|_| "the request is not JSON")?;
    let o = object(&v, "the request")?;
    let mut missing = Vec::new();
    let player = team(o.get("player"), "the player", &mut missing)?;
    let opponent = team(o.get("opponent"), "the opponent", &mut missing)?;
    let damage_cap = match o.get("damage_cap") {
        None | Some(Value::Null) => None,
        cap => Some(int(cap, "the damage cap", 1, MAX_CAP)?),
    };
    Ok(Request {
        player,
        opponent,
        damage_cap,
        missing,
    })
}
