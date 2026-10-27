//! Expected place (moonshot phase 3): how good a position was, turn by
//! turn. This crate turns a saved game record (the report the app saves,
//! T-104d) into one row of numbers per combat, the input of the model
//! (T-652); the model and its training come next (T-653).
//!
//! A row is the state the player could see as combat `t` began: the turn,
//! their health and the lobby's, their shop turn, both boards entering the
//! fight. Nothing new is read from the game: only what the record already
//! holds. A value the record does not have is `None` (unknown), never 0,
//! and the model sees it as missing. No player name, account or time is
//! read; the hero is kept as its card id for the model to learn per hero.
//!
//! The record is untrusted input: anything unexpected reads as unknown,
//! never as a panic.

use serde_json::Value;

/// The rows' format; raised whenever a feature is added, removed or changes
/// meaning, so a model is never fed rows it was not trained on.
pub const VERSION: u32 = 1;

/// The name of each feature, in row order.
pub const NAMES: [&str; 25] = [
    "solo",
    "turn",
    "health",
    "alive",
    "health_rank",
    "tavern_tier",
    "gold",
    "rolls",
    "buys",
    "minions",
    "attack_sum",
    "health_sum",
    "attack_max",
    "health_max",
    "golden",
    "taunt",
    "divine_shield",
    "reborn",
    "venomous",
    "windfury",
    "deathrattle",
    "opponent_minions",
    "opponent_attack_sum",
    "opponent_health_sum",
    "opponent_health",
];

/// Number of features in a row.
pub const WIDTH: usize = NAMES.len();

/// The keywords counted on the player's board, by the record's names, in
/// row order after `golden`.
const KEYWORDS: [&str; 6] = [
    "taunt",
    "divine_shield",
    "reborn",
    "venomous",
    "windfury",
    "deathrattle",
];

/// One combat's row.
#[derive(Clone, Debug, PartialEq)]
pub struct Turn {
    pub turn: i64,
    pub values: [Option<f64>; WIDTH],
}

/// One game's rows and what the model learns from them.
#[derive(Clone, Debug, PartialEq)]
pub struct Game {
    pub solo: bool,
    pub build: Option<i64>,
    /// The player's hero card id.
    pub hero: Option<String>,
    /// The place the game ended in (1 to 8 in Solo, 1 to 4 in Duos).
    pub final_place: i64,
    pub turns: Vec<Turn>,
}

/// Why a record gives no rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Skip {
    NotBattlegrounds,
    NoFinalPlace,
    NoCombats,
}

fn int(v: &Value, key: &str) -> Option<i64> {
    v.get(key)?.as_i64()
}

/// A per-player map of the record (`{"3": 40}`), read at a seat.
fn at_seat(map: Option<&Value>, seat: i64) -> Option<i64> {
    map?.as_object()?.get(&seat.to_string())?.as_i64()
}

/// Every seat's health in a per-player map.
fn healths(map: Option<&Value>) -> Vec<(i64, i64)> {
    map.and_then(Value::as_object)
        .map(|m| {
            m.iter()
                .filter_map(|(k, v)| Some((k.parse().ok()?, v.as_i64()?)))
                .collect()
        })
        .unwrap_or_default()
}

/// The board of the first entry on `side` (for the player's own side, the
/// local player's seat first), when the record has one.
fn board<'a>(entries: &'a [Value], side: &str, seat: Option<i64>) -> Option<&'a Vec<Value>> {
    let on_side = || {
        entries
            .iter()
            .filter(move |e| e.get("side").and_then(Value::as_str) == Some(side))
    };
    let mine = seat.and_then(|s| {
        on_side()
            .find(|e| int(e, "player_id") == Some(s))
            .and_then(|e| e.get("board")?.as_array())
    });
    mine.or_else(|| on_side().find_map(|e| e.get("board")?.as_array()))
}

fn opponent_seat(entries: &[Value]) -> Option<i64> {
    entries
        .iter()
        .find(|e| e.get("side").and_then(Value::as_str) == Some("opponent"))
        .and_then(|e| int(e, "player_id"))
}

/// Board features: minions, sums, maxima, golden and keyword counts. A
/// minion without readable stats makes the sums unknown.
fn board_values(board: &[Value]) -> [Option<f64>; 12] {
    let stats: Option<Vec<(i64, i64)>> = board
        .iter()
        .map(|m| Some((int(m, "atk")?, int(m, "health")?)))
        .collect();
    let count = |pred: &dyn Fn(&Value) -> bool| board.iter().filter(|m| pred(m)).count() as f64;
    let has = |m: &Value, k: &str| {
        m.get("keywords")
            .and_then(Value::as_array)
            .is_some_and(|ks| ks.iter().any(|x| x.as_str() == Some(k)))
    };
    let sum =
        |f: fn(&(i64, i64)) -> i64| stats.as_ref().map(|s| s.iter().map(f).sum::<i64>() as f64);
    let max = |f: fn(&(i64, i64)) -> i64| {
        stats
            .as_ref()
            .map(|s| s.iter().map(f).max().unwrap_or(0) as f64)
    };
    let mut out = [None; 12];
    out[0] = Some(board.len() as f64);
    out[1] = sum(|s| s.0);
    out[2] = sum(|s| s.1);
    out[3] = max(|s| s.0);
    out[4] = max(|s| s.1);
    out[5] = Some(count(&|m| {
        m.get("golden").and_then(Value::as_bool) == Some(true)
    }));
    for (i, k) in KEYWORDS.iter().enumerate() {
        out[6 + i] = Some(count(&|m| has(m, k)));
    }
    out
}

/// The shop turn numbered `t`, when the record has it.
fn shop_turn(report: &Value, t: i64) -> Option<&Value> {
    report
        .get("shop")?
        .get("turns")?
        .as_array()?
        .iter()
        .find(|s| int(s, "turn") == Some(t))
}

/// Health per seat as combat `t` began: after the round before it, or the
/// start of the game for the first.
fn healths_before<'a>(report: &'a Value, rounds: &'a [Value], i: usize) -> Option<&'a Value> {
    match i {
        0 => report.get("start_health"),
        _ => rounds[i - 1].get("health_after"),
    }
    .filter(|m| m.as_object().is_some_and(|o| !o.is_empty()))
}

fn turn_row(report: &Value, rounds: &[Value], i: usize, solo: bool) -> Option<Turn> {
    let round = &rounds[i];
    let t = int(round, "number")?;
    let entries: &[Value] = round.get("entries")?.as_array()?;
    let seat = int(report, "local_player_id");
    let own = board(entries, "own", seat)?;
    let mut v = [None; WIDTH];
    v[0] = Some(if solo { 1.0 } else { 0.0 });
    v[1] = Some(t as f64);
    let before = healths_before(report, rounds, i);
    let mine = seat.and_then(|s| at_seat(before, s));
    v[2] = mine.map(|h| h as f64);
    let all = healths(before);
    if !all.is_empty() {
        v[3] = Some(all.iter().filter(|(_, h)| *h > 0).count() as f64);
        v[4] = mine.map(|m| (all.iter().filter(|(_, h)| *h > m).count() + 1) as f64);
    }
    if let Some(s) = shop_turn(report, t) {
        v[5] = int(s, "tier").map(|x| x as f64);
        v[6] = int(s, "gold").map(|x| x as f64);
        v[7] = int(s, "rolls").map(|x| x as f64);
        v[8] = s
            .get("buys")
            .and_then(Value::as_array)
            .map(|b| b.len() as f64);
    }
    v[9..21].copy_from_slice(&board_values(own));
    if let Some(theirs) = board(entries, "opponent", None) {
        let b = board_values(theirs);
        v[21] = b[0];
        v[22] = b[1];
        v[23] = b[2];
    }
    v[24] = opponent_seat(entries)
        .and_then(|s| at_seat(before, s))
        .map(|h| h as f64);
    Some(Turn { turn: t, values: v })
}

/// The rows of one saved game record (its `report`), one per combat the
/// player's own board is known for, in round order.
pub fn encode(report: &Value) -> Result<Game, Skip> {
    let solo = match report.get("game_type").and_then(Value::as_str) {
        Some("GT_BATTLEGROUNDS") => true,
        Some("GT_BATTLEGROUNDS_DUO") => false,
        _ => return Err(Skip::NotBattlegrounds),
    };
    let places = if solo { 1..=8 } else { 1..=4 };
    let final_place = int(report, "final_place")
        .filter(|p| places.contains(p))
        .ok_or(Skip::NoFinalPlace)?;
    let rounds: &[Value] = report
        .get("rounds")
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice);
    let turns: Vec<Turn> = (0..rounds.len())
        .filter_map(|i| turn_row(report, rounds, i, solo))
        .collect();
    if turns.is_empty() {
        return Err(Skip::NoCombats);
    }
    Ok(Game {
        solo,
        build: int(report, "build"),
        hero: report.get("hero").and_then(Value::as_str).map(String::from),
        final_place,
        turns,
    })
}

/// The CSV header: an opaque game number, the game's build and hero,
/// the round, every feature (the mode and turn among them) and the final
/// place.
pub fn csv_header() -> String {
    let mut cols = vec!["game", "build", "hero", "round"];
    cols.extend(NAMES);
    cols.push("final_place");
    cols.join(",")
}

/// One game's rows as CSV lines (no header); an unknown value is an empty
/// cell. `game` is a running number, never an id from the record.
pub fn csv_rows(game: usize, g: &Game) -> Vec<String> {
    let cell = |v: &Option<f64>| v.map_or(String::new(), |x| format!("{x}"));
    let hero = g.hero.as_deref().filter(|h| is_card_id(h)).unwrap_or("");
    g.turns
        .iter()
        .map(|t| {
            let mut cols = vec![
                game.to_string(),
                g.build.map_or(String::new(), |b| b.to_string()),
                hero.to_string(),
                t.turn.to_string(),
            ];
            cols.extend(t.values.iter().map(cell));
            cols.push(g.final_place.to_string());
            cols.join(",")
        })
        .collect()
}

/// A card id as the record writes one: letters, digits and `_`, so a CSV
/// cell needs no quoting and holds nothing else.
fn is_card_id(s: &str) -> bool {
    !s.is_empty() && s.len() <= 64 && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

#[cfg(test)]
mod tests;
