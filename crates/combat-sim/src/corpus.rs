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
//! the keys kept, and [`check_private`] refuses a line with a key outside
//! its level's list, a string that is not a card id or a lower-case game
//! word, or a number outside its key's range, so a name, a BattleTag, an
//! account id or a time cannot pass.
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
/// The keys of a corpus line itself.
const LINE_KEYS: [&str; 5] = ["format", "id", "build", "mode", "report"];
/// The keys of its report.
const REPORT_KEYS: [&str; 3] = ["game_type", "build", "rounds"];
/// Every key a round may hold, at any depth: a round's, a board's, a
/// minion's, a kit's, the game-long values' and the replay's. A key outside
/// the list (a new field in a saved game) fails the export until someone
/// adds it here on purpose, so nothing new is published by accident.
#[rustfmt::skip]
const ROUND_TREE_KEYS: &[&str] = &[
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
/// Keys whose strings are card ids.
const CARD_ID_KEYS: [&str; 3] = ["card_id", "hero", "gifts"];
/// Keys whose numbers can be as big as a minion's stats (up to the 32-bit
/// limit, D-144); every other number is at most [`MAX_SMALL`].
const STAT_KEYS: [&str; 4] = ["atk", "health", "amount", "counters"];
/// The largest number a key outside [`STAT_KEYS`] holds (the saved game's
/// largest game-long value, `saved::MAX_GAME_VALUE`).
const MAX_SMALL: i64 = 99_999;
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

/// Refuses a value that is not a corpus line's shape: a key outside the
/// lists of its level ([`LINE_KEYS`], [`REPORT_KEYS`], [`ROUND_TREE_KEYS`]),
/// a string under a card id key that is not a card id (letters and digits
/// with at least one `_`), any other string that is not a lower-case game
/// word (`divine_shield`), a number that is not a whole number in its
/// key's range. A name, a BattleTag, an account id or a time cannot pass.
/// The error names where, never the value.
pub fn check_private(v: &Value) -> Result<(), String> {
    check_level(v, "", &LINE_KEYS)
}

fn check_level(v: &Value, path: &str, keys: &[&str]) -> Result<(), String> {
    let map = v
        .as_object()
        .ok_or_else(|| format!("{path}: not an object"))?;
    map.iter().try_for_each(|(k, v)| {
        if !keys.contains(&k.as_str()) {
            return Err(format!("{path}/{k}: a key that is not one a corpus line keeps"));
        }
        let here = format!("{path}/{k}");
        match k.as_str() {
            "report" => check_level(v, &here, &REPORT_KEYS),
            "rounds" => v
                .as_array()
                .ok_or_else(|| format!("{here}: not a list"))?
                .iter()
                .try_for_each(|r| check_tree(r, &here, "rounds")),
            _ => check_tree(v, &here, k),
        }
    })
}

/// A value under `key`, inside a round or as a scalar of the line.
fn check_tree(v: &Value, path: &str, key: &str) -> Result<(), String> {
    match v {
        Value::Array(items) => items.iter().try_for_each(|i| check_tree(i, path, key)),
        Value::Object(map) => map.iter().try_for_each(|(k, v)| {
            if !ROUND_TREE_KEYS.contains(&k.as_str()) {
                return Err(format!("{path}/{k}: a key that is not one a corpus line keeps"));
            }
            check_tree(v, &format!("{path}/{k}"), k)
        }),
        Value::String(s) => {
            let ok = s.len() <= MAX_STRING
                && match key {
                    k if CARD_ID_KEYS.contains(&k) => is_card_id(s),
                    "game_type" => s == "GT_BATTLEGROUNDS" || s == "GT_BATTLEGROUNDS_DUO",
                    "id" => s.len() == 16 && s.bytes().all(|b| b.is_ascii_hexdigit()),
                    _ => is_word(s),
                };
            ok.then_some(())
                .ok_or_else(|| format!("{path}: a string that is not a card id or a game word"))
        }
        Value::Number(n) => {
            let max = match key {
                k if STAT_KEYS.contains(&k) => i64::from(i32::MAX),
                "build" => 999_999_999,
                _ => MAX_SMALL,
            };
            n.as_i64()
                .filter(|n| n.abs() <= max)
                .map(|_| ())
                .ok_or_else(|| format!("{path}: a number out of range or not whole"))
        }
        Value::Bool(_) | Value::Null => Ok(()),
    }
}

/// A card id: starts with a letter, letters, digits and `_`, at least one
/// `_` (`BG20_GEM`, `TB_BaconShop_HERO_35_SKIN_E`).
fn is_card_id(s: &str) -> bool {
    s.starts_with(|c: char| c.is_ascii_alphabetic())
        && s.contains('_')
        && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// A game word: lower-case letters and `_` (`own`, `start_of_combat`).
fn is_word(s: &str) -> bool {
    s.starts_with(|c: char| c.is_ascii_lowercase())
        && s.len() <= 32
        && s.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
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
                Ok((id, line)) => match lines.get(&id) {
                    None => {
                        lines.insert(id, line);
                        *out.written.entry((build, mode_of(game_type))).or_default() += 1;
                    }
                    Some(same) if *same == line => {
                        *out.skipped.entry("same combat twice").or_default() += 1;
                    }
                    Some(_) => return Err(format!("two different combats share the id {id}")),
                },
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
    let game_type = report.get("game_type").and_then(Value::as_str);
    if game_type
        != Some(if duos {
            "GT_BATTLEGROUNDS_DUO"
        } else {
            "GT_BATTLEGROUNDS"
        })
    {
        return Err("the report's game type is not the line's mode".into());
    }
    if report.get("build").and_then(Value::as_u64) != Some(build) {
        return Err("the report's build is not the line's".into());
    }
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

/// The most a compressed corpus may unpack to (the maintainer's 503
/// combats are about 4 MB).
const MAX_CORPUS_BYTES: u64 = 256 * 1024 * 1024;

/// A corpus file, gzip-compressed when its name ends in `.gz`; one that
/// unpacks past [`MAX_CORPUS_BYTES`] is cut there and fails to read.
pub fn read_corpus_file(path: &Path) -> Result<Vec<CorpusCombat>, String> {
    use std::io::Read as _;
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let text = if path.extension().is_some_and(|x| x == "gz") {
        let mut text = String::new();
        flate2::read::GzDecoder::new(&bytes[..])
            .take(MAX_CORPUS_BYTES)
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
