//! The conformance corpus (T-640): every logged combat as one frozen test
//! case, so the engine can be checked against the game on every change
//! without the player's history.
//!
//! One JSON line per combat:
//!
//! ```text
//! {"format": 1, "id": "<16 hex>", "build": 254477, "mode": "solo" | "duos",
//!  "report": {"game_type": ..., "build": ..., "rounds": [<one round>]}}
//! ```
//!
//! The round keeps only what the engine reads: its number, the result, the
//! entries (side, seat, hero card id, kit, board, game-long values) and the
//! saved replay. Everything else a saved game holds (names, the lobby, the
//! session string, when it was played, the shop) is left out by a list of
//! the keys kept, and [`check_private`] refuses a line with any key outside
//! one list or any string that is not a card id or a game word, so a name,
//! a BattleTag or a time cannot pass.
//!
//! The card data is not in the corpus (HearthstoneJSON's data stays out of
//! the repository): a reader gives each combat the facts of its own build's
//! `cards.json`, fetched at run time. The id is an FNV-1a hash of the
//! line's canonical form, so a combat keeps its id across exports and two
//! copies of one combat merge.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::Path;

use serde_json::{Map, Value};

use crate::harness::latest_reports;

/// The corpus line's format; a reader refuses any other.
pub const FORMAT: u64 = 1;

/// The keys a corpus round keeps.
const ROUND_KEYS: [&str; 4] = ["number", "result", "entries", "replay"];
/// The keys a corpus entry (one board in a round) keeps.
const ENTRY_KEYS: [&str; 6] = ["side", "player_id", "hero", "kit", "board", "values"];
/// Every key a corpus line may hold, at any depth: the line's own, the
/// report's, a round's, a board's, a minion's, a kit's, the game-long
/// values' and the replay's. A key outside the list (a new field in a saved
/// game) fails the export until someone adds it here on purpose, so nothing
/// new is published by accident.
#[rustfmt::skip]
const KEYS: &[&str] = &[
    // line and report
    "format", "id", "build", "mode", "report", "game_type", "rounds",
    // round and entry
    "number", "result", "entries", "replay", "side", "player_id", "hero", "kit", "board", "values",
    // minion
    "atk", "card_id", "golden", "health", "position", "keywords", "gifts", "gift_unknown", "buffs",
    "count", "counters",
    // kit
    "powers", "trinkets", "slot", "target", "held",
    // game-long values
    "blood_gem_atk", "blood_gem_health", "spell_atk", "spell_health", "knights_dead",
    "undead_atk", "beetle_atk", "beetle_health",
    // replay
    "complete", "events", "minions", "start", "beats", "kind", "source", "trigger", "minion", "on",
    "amount", "attacker", "defender", "keyword", "how",
];
/// The longest string a corpus line holds (card ids are about 30).
const MAX_STRING: usize = 64;

/// One combat read from the corpus.
#[derive(Clone, Debug, PartialEq)]
pub struct CorpusCombat {
    pub id: String,
    pub build: u64,
    pub duos: bool,
    /// A one-round report, read by [`crate::saved`] like a saved game.
    pub report: Value,
}

impl CorpusCombat {
    /// The combat's round as saved.
    pub fn round(&self) -> &Value {
        &self.report["rounds"][0]
    }
}

/// What an export wrote and left out.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Export {
    /// Corpus lines, sorted by id, one per distinct combat.
    pub lines: Vec<String>,
    /// Combats left out, by reason.
    pub skipped: BTreeMap<&'static str, usize>,
    /// Combats written per (build, mode).
    pub written: BTreeMap<(u64, &'static str), usize>,
    pub games: usize,
    pub bad_lines: usize,
}

/// FNV-1a, 64 bits.
pub fn fnv64(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// The value as JSON with every object's keys sorted, whatever order the
/// map keeps them in (serde_json's `preserve_order` feature can be on in
/// some builds of the workspace).
pub fn canonical(v: &Value) -> String {
    let mut out = String::new();
    write_canonical(v, &mut out);
    out
}

fn write_canonical(v: &Value, out: &mut String) {
    match v {
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_canonical(item, out);
            }
            out.push(']');
        }
        Value::Object(map) => {
            let keys: BTreeSet<&String> = map.keys().collect();
            out.push('{');
            for (i, k) in keys.into_iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&Value::String(k.clone()).to_string());
                out.push(':');
                write_canonical(&map[k], out);
            }
            out.push('}');
        }
        other => out.push_str(&other.to_string()),
    }
}

/// Refuses a value that holds a key outside [`KEYS`] or a string that is
/// not an identifier (letters, digits and `_`, at most [`MAX_STRING`]): a
/// name, a BattleTag, a time or free text cannot pass.
pub fn check_private(v: &Value) -> Result<(), String> {
    match v {
        Value::Array(items) => items.iter().try_for_each(check_private),
        Value::Object(map) => map.iter().try_for_each(|(k, v)| {
            if !KEYS.contains(&k.as_str()) {
                return Err(format!("key \"{k}\" is not one a corpus line keeps"));
            }
            check_private(v)
        }),
        Value::String(s) => {
            let plain = s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_');
            if s.is_empty() || s.len() > MAX_STRING || !plain {
                return Err(format!("\"{s}\" is not a card id or a game word"));
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn keep(v: &Value, keys: &[&str]) -> Option<Value> {
    let map = v.as_object()?;
    let kept: Map<String, Value> = keys
        .iter()
        .filter_map(|k| map.get(*k).map(|v| ((*k).to_string(), v.clone())))
        .collect();
    Some(Value::Object(kept))
}

fn build_of(report: &Value) -> Option<u64> {
    match report.get("build")? {
        Value::Number(n) => n.as_u64(),
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
    .filter(|b| (1..1_000_000_000).contains(b))
}

fn mode_of(game_type: &str) -> &'static str {
    if game_type == "GT_BATTLEGROUNDS" {
        "solo"
    } else {
        "duos"
    }
}

/// Why a round is not a corpus line.
enum Skip {
    /// Left out, counted under this reason.
    Out(&'static str),
    /// It would carry something [`check_private`] refuses: the export stops.
    Refused(String),
}

/// One round as a corpus line: its id and the line.
fn line_of(round: &Value, game_type: &str, build: u64) -> Result<(String, String), Skip> {
    let out = |why| Err(Skip::Out(why));
    if crate::saved::result(round).is_none() {
        return out("no result");
    }
    if round.get("number").and_then(Value::as_i64).is_none() {
        return out("no round number");
    }
    let Some(mut kept) = keep(round, &ROUND_KEYS) else {
        return out("not a round");
    };
    let entries: Vec<Value> = round
        .get("entries")
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice)
        .iter()
        .filter_map(|e| keep(e, &ENTRY_KEYS))
        .collect();
    if entries.is_empty() {
        return out("no boards");
    }
    kept["entries"] = Value::Array(entries);
    let mut line = serde_json::json!({
        "format": FORMAT,
        "build": build,
        "mode": mode_of(game_type),
        "report": {"game_type": game_type, "build": build, "rounds": [kept]},
    });
    check_private(&line).map_err(Skip::Refused)?;
    let id = format!("{:016x}", fnv64(&canonical(&line)));
    line["id"] = Value::String(id.clone());
    Ok((id, canonical(&line)))
}

/// Every combat with a result in a history (the app's `games.jsonl`), as
/// corpus lines. An error names a combat that would carry something
/// [`check_private`] refuses.
pub fn export(history: &str) -> Result<Export, String> {
    let (reports, bad_lines) = latest_reports(history);
    let mut out = Export {
        games: reports.len(),
        bad_lines,
        ..Export::default()
    };
    let mut lines: BTreeMap<String, String> = BTreeMap::new();
    for report in &reports {
        let game_type = match report.get("game_type").and_then(Value::as_str) {
            Some(t @ ("GT_BATTLEGROUNDS" | "GT_BATTLEGROUNDS_DUO")) => t,
            _ => {
                *out.skipped.entry("not a Battlegrounds game").or_default() += 1;
                continue;
            }
        };
        let Some(build) = build_of(report) else {
            *out.skipped.entry("game without a build").or_default() += 1;
            continue;
        };
        let rounds = report.get("rounds").and_then(Value::as_array);
        for round in rounds.into_iter().flatten() {
            match line_of(round, game_type, build) {
                Ok((id, line)) => {
                    if lines.insert(id, line).is_none() {
                        *out.written.entry((build, mode_of(game_type))).or_default() += 1;
                    } else {
                        *out.skipped.entry("same combat twice").or_default() += 1;
                    }
                }
                Err(Skip::Out(why)) => *out.skipped.entry(why).or_default() += 1,
                Err(Skip::Refused(why)) => {
                    return Err(format!("a combat of build {build} was refused: {why}"))
                }
            }
        }
    }
    out.lines = lines.into_values().collect();
    Ok(out)
}

/// One corpus line, checked: its format, its id against its content, and
/// that it holds nothing private.
pub fn read_line(line: &str) -> Result<CorpusCombat, String> {
    let mut v: Value = serde_json::from_str(line).map_err(|e| format!("not JSON: {e}"))?;
    if v.get("format").and_then(Value::as_u64) != Some(FORMAT) {
        return Err(format!("format is not {FORMAT}"));
    }
    check_private(&v)?;
    let id = v
        .as_object_mut()
        .and_then(|m| m.remove("id"))
        .and_then(|id| id.as_str().map(String::from))
        .ok_or("no id")?;
    let want = format!("{:016x}", fnv64(&canonical(&v)));
    if id != want {
        return Err(format!("id {id} does not match its content ({want})"));
    }
    let build = v.get("build").and_then(Value::as_u64).ok_or("no build")?;
    let duos = match v.get("mode").and_then(Value::as_str) {
        Some("solo") => false,
        Some("duos") => true,
        _ => return Err("mode is not solo or duos".into()),
    };
    let report = v.get("report").cloned().ok_or("no report")?;
    let rounds = report.get("rounds").and_then(Value::as_array).map(Vec::len);
    if rounds != Some(1) {
        return Err("the report does not hold exactly one round".into());
    }
    Ok(CorpusCombat {
        id,
        build,
        duos,
        report,
    })
}

/// A whole corpus; an error names the first bad line (1-based) or a
/// repeated id.
pub fn read_corpus(text: &str) -> Result<Vec<CorpusCombat>, String> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let c = read_line(line).map_err(|e| format!("corpus line {}: {e}", n + 1))?;
        if !seen.insert(c.id.clone()) {
            return Err(format!("corpus line {}: id {} repeated", n + 1, c.id));
        }
        out.push(c);
    }
    Ok(out)
}

/// A corpus file, gzip-compressed when its name ends in `.gz`.
pub fn read_corpus_file(path: &Path) -> Result<Vec<CorpusCombat>, String> {
    use std::io::Read as _;
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let text = if path.extension().is_some_and(|x| x == "gz") {
        let mut text = String::new();
        flate2::read::GzDecoder::new(&bytes[..])
            .read_to_string(&mut text)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        text
    } else {
        String::from_utf8(bytes).map_err(|e| format!("{}: {e}", path.display()))?
    };
    read_corpus(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// The export's counts, one line each: no game, round or card.
pub fn render_export(e: &Export) -> String {
    let mut out = String::new();
    let total: usize = e.written.values().sum();
    let _ = writeln!(
        out,
        "games read: {} (lines not a game: {})",
        e.games, e.bad_lines
    );
    let _ = writeln!(out, "combats written: {total}");
    for ((build, mode), n) in &e.written {
        let _ = writeln!(out, "  build {build} {mode}: {n}");
    }
    for (why, n) in &e.skipped {
        let _ = writeln!(out, "left out ({why}): {n}");
    }
    out
}

#[cfg(test)]
#[path = "corpus_tests.rs"]
mod tests;
