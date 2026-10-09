//! What one game looks like after parsing. Serializes to the same JSON as
//! `tools/parse_bg.py --json`, so both can be compared field by field.
//!
//! Privacy: only card ids and lobby player ids (1-8), never names or accounts.

use std::collections::BTreeMap;

use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};

pub const NOT_IN_POWER_LOG: [&str; 2] = ["MMR", "available tribes (exact list)"];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Ok,
    Incomplete,
    Unsupported,
    NotBattlegrounds,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Own,
    Opponent,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Minion {
    pub card_id: Option<String>,
    pub atk: i64,
    pub health: i64,
    pub position: i64,
    pub golden: bool,
}

/// One side stepping into a combat. `board` is None when the fight never
/// plays out in the local log: unknown, not empty.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CombatEntry {
    pub side: Side,
    pub player_id: Option<i64>,
    pub hero: Option<String>,
    pub board: Option<Vec<Minion>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Round {
    pub number: i64,
    pub entries: Vec<CombatEntry>,
    pub own_health_after: Option<i64>,
    /// Health (health + armor - damage, never below 0) of every lobby hero by
    /// player id once the combat closed. Empty when the log did not give it
    /// (unknown, not zero). Rust-only: the Python prototype does not write it.
    pub health_after: BTreeMap<i64, i64>,
}

impl Round {
    pub fn opponents(&self) -> Vec<Option<i64>> {
        self.entries
            .iter()
            .filter(|e| e.side == Side::Opponent)
            .map(|e| e.player_id)
            .collect()
    }
}

impl Serialize for Round {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut map = s.serialize_map(Some(5))?;
        map.serialize_entry("number", &self.number)?;
        map.serialize_entry("entries", &self.entries)?;
        map.serialize_entry("own_health_after", &self.own_health_after)?;
        map.serialize_entry("opponents", &self.opponents())?;
        map.serialize_entry("health_after", &self.health_after)?;
        map.end()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LobbyPlayer {
    pub player_id: i64,
    pub hero: Option<String>,
    pub duo_team: Option<i64>,
    pub final_place: Option<i64>,
    pub final_health: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GameReport {
    pub index: usize,
    pub status: Status,
    pub game_type: Option<String>,
    pub build: Option<i64>,
    pub local_player_id: Option<i64>,
    pub hero: Option<String>,
    pub teammate_player_id: Option<i64>,
    pub teammate_hero: Option<String>,
    /// Card id -> name, for the heroes in this report whose name the log
    /// prints (in the game client's language). Not in the Python prototype.
    pub card_names: BTreeMap<String, String>,
    pub final_place: Option<i64>,
    pub final_health: Option<i64>,
    pub lobby: Vec<LobbyPlayer>,
    /// Tribe name and how many shop offers had it, most common first.
    #[serde(serialize_with = "pairs_as_map")]
    pub shop_tribes: Vec<(String, usize)>,
    pub rounds: Vec<Round>,
    /// Health of every lobby hero by player id before the first combat. Empty
    /// when the log did not give it. Rust-only, like `Round::health_after`.
    pub start_health: BTreeMap<i64, i64>,
    pub warnings: Vec<String>,
    pub problems: Vec<String>,
    pub not_in_log: Vec<String>,
}

impl GameReport {
    pub fn new(
        index: usize,
        status: Status,
        game_type: Option<String>,
        build: Option<i64>,
    ) -> Self {
        GameReport {
            index,
            status,
            game_type,
            build,
            local_player_id: None,
            hero: None,
            teammate_player_id: None,
            teammate_hero: None,
            card_names: BTreeMap::new(),
            final_place: None,
            final_health: None,
            lobby: Vec::new(),
            shop_tribes: Vec::new(),
            rounds: Vec::new(),
            start_health: BTreeMap::new(),
            warnings: Vec::new(),
            problems: Vec::new(),
            not_in_log: NOT_IN_POWER_LOG.iter().map(|s| s.to_string()).collect(),
        }
    }
}

fn pairs_as_map<S: Serializer>(pairs: &[(String, usize)], s: S) -> Result<S::Ok, S::Error> {
    let mut map = s.serialize_map(Some(pairs.len()))?;
    for (k, v) in pairs {
        map.serialize_entry(k, v)?;
    }
    map.end()
}

fn or_dash<T: ToString>(value: Option<T>) -> String {
    value.map(|v| v.to_string()).unwrap_or_else(|| "-".into())
}

fn opt<T: ToString>(value: &Option<T>) -> String {
    value
        .as_ref()
        .map(|v| v.to_string())
        .unwrap_or_else(|| "None".into())
}

fn fmt_board(board: &Option<Vec<Minion>>) -> String {
    match board {
        None => "(not visible in the log)".into(),
        Some(b) if b.is_empty() => "(empty)".into(),
        Some(b) => b
            .iter()
            .map(|m| {
                format!(
                    "{} {}/{}{}",
                    opt(&m.card_id),
                    m.atk,
                    m.health,
                    if m.golden { " golden" } else { "" }
                )
            })
            .collect::<Vec<_>>()
            .join(", "),
    }
}

fn fmt_ids(ids: &[Option<i64>]) -> String {
    let items: Vec<String> = ids.iter().map(opt).collect();
    format!("[{}]", items.join(", "))
}

/// Human-readable summary, same wording as the Python prototype.
pub fn format_text(r: &GameReport) -> String {
    let status = serde_json::to_value(r.status)
        .ok()
        .and_then(|v| v.as_str().map(String::from));
    let mut lines = vec![format!(
        "Game {}: {}, build {} - {}",
        r.index,
        r.game_type.as_deref().unwrap_or("unknown type"),
        r.build
            .map(|b| b.to_string())
            .unwrap_or_else(|| "unknown".into()),
        status.unwrap_or_default()
    )];
    lines.extend(r.warnings.iter().map(|w| format!("  warning: {w}")));
    lines.extend(r.problems.iter().map(|p| format!("  problem: {p}")));
    if !matches!(r.status, Status::Ok | Status::Incomplete) {
        return lines.join("\n");
    }
    let mate = match r.teammate_player_id {
        Some(pid) => format!("; teammate {} (player {pid})", opt(&r.teammate_hero)),
        None => String::new(),
    };
    let place = r
        .final_place
        .map(|p| p.to_string())
        .unwrap_or_else(|| "not available".into());
    lines.push(format!(
        "  Hero: {} (player {}){mate}",
        opt(&r.hero),
        opt(&r.local_player_id)
    ));
    lines.push(format!(
        "  Result: place {place}, health {}",
        opt(&r.final_health)
    ));
    let lobby: Vec<String> = r
        .lobby
        .iter()
        .map(|p| {
            format!(
                "p{} {} team {} place {}",
                p.player_id,
                opt(&p.hero),
                or_dash(p.duo_team),
                or_dash(p.final_place)
            )
        })
        .collect();
    lines.push(format!("  Lobby: {}", lobby.join("; ")));
    let tribes: Vec<String> = r
        .shop_tribes
        .iter()
        .map(|(n, c)| format!("{n} {c}"))
        .collect();
    let tribes = if tribes.is_empty() {
        "none seen".into()
    } else {
        tribes.join(", ")
    };
    lines.push(format!(
        "  Tribes seen in shop (inferred, not the exact lobby list): {tribes}"
    ));
    lines.push("  MMR: not available in Power.log".into());
    for round in &r.rounds {
        lines.push(format!(
            "  Round {}: health after {}, opponents {}",
            round.number,
            opt(&round.own_health_after),
            fmt_ids(&round.opponents())
        ));
        for e in &round.entries {
            let side = if e.side == Side::Own {
                "own"
            } else {
                "opponent"
            };
            lines.push(format!(
                "    {side} p{} {}: {}",
                opt(&e.player_id),
                opt(&e.hero),
                fmt_board(&e.board)
            ));
        }
    }
    lines.join("\n")
}
