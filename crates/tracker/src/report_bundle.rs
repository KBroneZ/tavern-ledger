//! "Report a problem" bundle (T-110): one JSON file about one game, with
//! the parser version, the game build, the app version, the log setup check
//! and the parser's warnings and problems. The user reads it and sends it by
//! hand; nothing here touches the network.
//!
//! Names stay out in two ways. The bundle is made only of fields in an
//! allowed set (`BUNDLE`), and a check refuses it, whole, if a field is
//! outside that set or a text looks like a BattleTag or an account id. A
//! refusal names the field, never its value.

use std::fmt;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::setup::{FileCheck, SetupReport};
use crate::store::ParserStamp;

pub const FORMAT: &str = "tavern-ledger-problem-report";
pub const FORMAT_VERSION: u32 = 1;

/// A text longer than this is not something the parser wrote.
const MAX_TEXT: usize = 2_000;
/// Shortest run of digits after `#` that makes a text look like a BattleTag.
const TAG_DIGITS: usize = 3;
const MAX_FILE_COPIES: u32 = 99;

/// Longest id (card ids, tribes, session folders) the parser can write.
const MAX_ID: usize = 100;

/// What a field may hold. A `null` is fine for every shape.
enum Shape {
    /// A number, a bool or a short text.
    Scalar,
    /// A card id, a tribe, a status: letters, digits and `_` only, so no
    /// free text (a name) fits.
    Id,
    /// Keys that are ids, with scalar values (card ids to names, tribes to counts).
    Map,
    List(&'static Shape),
    /// Only these keys.
    Object(&'static [(&'static str, Shape)]),
    /// A scalar, or an object of this shape.
    ScalarOr(&'static Shape),
}

const MINION: Shape = Shape::Object(&[
    ("card_id", Shape::Id),
    ("atk", Shape::Scalar),
    ("health", Shape::Scalar),
    ("position", Shape::Scalar),
    ("golden", Shape::Scalar),
    // Parser revision 5: keyword names.
    ("keywords", Shape::List(&Shape::Id)),
    // Parser revision 9: Dark Gifts, by card id.
    ("gifts", Shape::List(&Shape::Id)),
    ("gift_unknown", Shape::Scalar),
]);

const ENTRY: Shape = Shape::Object(&[
    ("side", Shape::Id),
    ("player_id", Shape::Scalar),
    ("hero", Shape::Id),
    ("board", Shape::List(&MINION)),
    // Parser revision 10: the side's game-long values (numbers only).
    ("values", Shape::Map),
]);

// Parser revision 8 (T-215, D-099): each shown combat's replay.
const REPLAY_MINION: Shape = Shape::Object(&[
    ("side", Shape::Id),
    ("card_id", Shape::Id),
    ("atk", Shape::Scalar),
    ("health", Shape::Scalar),
    ("position", Shape::Scalar),
    ("golden", Shape::Scalar),
    ("keywords", Shape::List(&Shape::Id)),
    ("gifts", Shape::List(&Shape::Id)),
    ("gift_unknown", Shape::Scalar),
]);

/// Where a damage came from (parser revision 13, T-575).
const DAMAGE_SOURCE: Shape = Shape::Object(&[
    ("how", Shape::Id),
    ("minion", Shape::Scalar),
    ("card_id", Shape::Id),
    ("side", Shape::Id),
]);

/// Every key any replay event kind has: indexes and numbers, or ids. A
/// trigger's source is a minion's index, a damage's is an object.
const REPLAY_EVENT: Shape = Shape::Object(&[
    ("kind", Shape::Id),
    ("attacker", Shape::Scalar),
    ("defender", Shape::Scalar),
    ("minion", Shape::Scalar),
    ("amount", Shape::Scalar),
    ("health", Shape::Scalar),
    ("on", Shape::Scalar),
    ("atk", Shape::Scalar),
    ("trigger", Shape::Id),
    ("side", Shape::Id),
    ("card_id", Shape::Id),
    ("source", Shape::ScalarOr(&DAMAGE_SOURCE)),
    ("hero", Shape::Id),
    ("keyword", Shape::Id),
]);

const REPLAY: Shape = Shape::Object(&[
    ("minions", Shape::List(&REPLAY_MINION)),
    ("start", Shape::Scalar),
    ("events", Shape::List(&REPLAY_EVENT)),
    ("complete", Shape::Scalar),
    ("beats", Shape::List(&Shape::Scalar)),
]);

const ROUND: Shape = Shape::Object(&[
    ("number", Shape::Scalar),
    ("entries", Shape::List(&ENTRY)),
    ("own_health_after", Shape::Scalar),
    ("opponents", Shape::List(&Shape::Scalar)),
    ("health_after", Shape::Map),
    ("result", Shape::Id),
    ("replay", REPLAY),
]);

const LOBBY_PLAYER: Shape = Shape::Object(&[
    ("player_id", Shape::Scalar),
    ("hero", Shape::Id),
    ("duo_team", Shape::Scalar),
    ("final_place", Shape::Scalar),
    ("final_health", Shape::Scalar),
    // Parser revision 5: the leaderboard's triples and trinkets.
    (
        "triples",
        Shape::List(&Shape::Object(&[
            ("turn", Shape::Scalar),
            ("count", Shape::Scalar),
        ])),
    ),
    (
        "trinkets",
        Shape::List(&Shape::Object(&[
            ("turn", Shape::Scalar),
            ("slot", Shape::Id),
            ("dbf_id", Shape::Scalar),
        ])),
    ),
]);

/// The player's own choices (parser revision 5).
const DISCOVER: Shape = Shape::Object(&[
    ("turn", Shape::Scalar),
    ("source", Shape::Id),
    (
        "offered",
        Shape::List(&Shape::Object(&[
            ("card_id", Shape::Id),
            ("gift", Shape::Id),
        ])),
    ),
    ("picked", Shape::List(&Shape::Scalar)),
]);

const TEXTS: Shape = Shape::List(&Shape::Scalar);

const OFFER: Shape = Shape::Object(&[
    ("card_id", Shape::Id),
    ("roll", Shape::Scalar),
    ("frozen", Shape::Scalar),
]);

const SHOP_TURN: Shape = Shape::Object(&[
    ("turn", Shape::Scalar),
    ("start_ms", Shape::Scalar),
    ("end_ms", Shape::Scalar),
    ("tier", Shape::Scalar),
    ("gold", Shape::Scalar),
    ("gold_spent", Shape::Scalar),
    ("rolls", Shape::Scalar),
    ("free_rolls", Shape::Scalar),
    ("buys", Shape::List(&Shape::Id)),
    ("spell_buys", Shape::Scalar),
    ("sells", Shape::List(&Shape::Id)),
    ("freezes", Shape::Scalar),
    ("offers", Shape::List(&OFFER)),
    ("actions", Shape::Scalar),
    ("extra_gold", Shape::Scalar),
    ("sell_gold", Shape::Scalar),
    ("buy_gold", Shape::Scalar),
    ("spell_gold", Shape::Scalar),
    ("free_refreshes", Shape::Scalar),
    ("passes", Shape::List(&Shape::Id)),
]);

/// The player's own hero pick (parser revision 4).
const HERO_SELECT: Shape = Shape::Object(&[
    ("offered", Shape::List(&Shape::Id)),
    ("rerolls", Shape::Scalar),
    ("picked", Shape::Id),
]);

/// The player's own shop and logged actions (parser revision 3).
const SHOP: Shape = Shape::Object(&[
    ("turns", Shape::List(&SHOP_TURN)),
    (
        "tier_ups",
        Shape::List(&Shape::Object(&[
            ("turn", Shape::Scalar),
            ("tier", Shape::Scalar),
        ])),
    ),
    (
        "actions",
        Shape::List(&Shape::Object(&[
            ("ms", Shape::Scalar),
            ("turn", Shape::Scalar),
            ("kind", Shape::Id),
        ])),
    ),
    ("start_ms", Shape::Scalar),
    ("end_ms", Shape::Scalar),
    ("ended", Shape::Scalar),
]);

/// The game report, as `bg_parser::report::GameReport` serializes it.
const REPORT: Shape = Shape::Object(&[
    ("index", Shape::Scalar),
    ("status", Shape::Id),
    ("game_type", Shape::Id),
    ("build", Shape::Scalar),
    ("local_player_id", Shape::Scalar),
    ("hero", Shape::Id),
    ("teammate_player_id", Shape::Scalar),
    ("teammate_hero", Shape::Id),
    ("card_names", Shape::Map),
    ("final_place", Shape::Scalar),
    ("final_health", Shape::Scalar),
    ("lobby", Shape::List(&LOBBY_PLAYER)),
    ("shop_tribes", Shape::Map),
    ("rounds", Shape::List(&ROUND)),
    ("start_health", Shape::Map),
    ("shop", SHOP),
    ("hero_select", HERO_SELECT),
    ("skin_parents", Shape::Map),
    ("discovers", Shape::List(&DISCOVER)),
    ("old_god", Shape::Scalar),
    ("warnings", TEXTS),
    ("problems", TEXTS),
    ("not_in_log", TEXTS),
]);

const PARSER: Shape = Shape::Object(&[("version", Shape::Scalar), ("revision", Shape::Scalar)]);

const CONFIG_CHECK: Shape = Shape::Object(&[
    ("state", Shape::Id),
    ("error_kind", Shape::Id),
    ("missing", TEXTS),
]);

/// Everything the bundle may contain.
const BUNDLE: Shape = Shape::Object(&[
    ("format", Shape::Scalar),
    ("format_version", Shape::Scalar),
    ("created_unix", Shape::Scalar),
    ("app_version", Shape::Scalar),
    ("parser", PARSER),
    ("parser_now", PARSER),
    (
        "game",
        Shape::Object(&[
            ("session", Shape::Id),
            ("index", Shape::Scalar),
            ("build", Shape::Scalar),
        ]),
    ),
    (
        "log_setup",
        Shape::Object(&[
            ("log_config", CONFIG_CHECK),
            ("client_config", CONFIG_CHECK),
        ]),
    ),
    ("warnings", TEXTS),
    ("problems", TEXTS),
    ("report", REPORT),
]);

/// Why a bundle was not made. Holds field paths only, never values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// A field outside the allowed set.
    UnknownField(String),
    /// A field with the wrong kind of value (an object where a number goes).
    WrongKind(String),
    /// A text that looks like a BattleTag or an account id.
    NameShaped(String),
    /// A text too long to be one the parser wrote.
    TooLong(String),
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Refusal::UnknownField(p) => write!(f, "field outside the allowed set: {p}"),
            Refusal::WrongKind(p) => write!(f, "unexpected kind of value in: {p}"),
            Refusal::NameShaped(p) => write!(f, "text that looks like a player name in: {p}"),
            Refusal::TooLong(p) => write!(f, "text too long in: {p}"),
        }
    }
}

impl std::error::Error for Refusal {}

/// Characters that read as `#`, including the full-width and small forms.
const HASHES: [char; 3] = ['#', '\u{ff03}', '\u{fe5f}'];

/// A `#` followed by digits of any script: `Name#1234`. What comes before
/// it does not matter (a name can end in a space, `_` or a mark).
fn looks_like_battletag(text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    chars.iter().enumerate().any(|(i, c)| {
        HASHES.contains(c)
            && chars[i + 1..].iter().take_while(|d| d.is_numeric()).count() >= TAG_DIGITS
    })
}

/// The log writes account ids as `[hi=… lo=…]` and `GameAccountId`.
fn looks_like_account_id(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("gameaccountid") || (lower.contains("hi=") && lower.contains("lo="))
}

fn check_text(text: &str, path: &str) -> Result<(), Refusal> {
    if text.chars().count() > MAX_TEXT {
        return Err(Refusal::TooLong(path.to_string()));
    }
    if looks_like_battletag(text) || looks_like_account_id(text) {
        return Err(Refusal::NameShaped(path.to_string()));
    }
    Ok(())
}

fn check_id(text: &str, path: &str) -> Result<(), Refusal> {
    let plain = !text.is_empty()
        && text.len() <= MAX_ID
        && text.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if plain {
        return Ok(());
    }
    // A name-shaped text is named as such; the path never holds the text.
    check_text(text, path)?;
    Err(Refusal::WrongKind(path.to_string()))
}

fn check_scalar(value: &Value, path: &str) -> Result<(), Refusal> {
    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) => Ok(()),
        Value::String(s) => check_text(s, path),
        _ => Err(Refusal::WrongKind(path.to_string())),
    }
}

fn check(value: &Value, shape: &Shape, path: &str) -> Result<(), Refusal> {
    if value.is_null() {
        return Ok(());
    }
    match shape {
        Shape::Scalar => check_scalar(value, path),
        Shape::ScalarOr(inner) if value.is_object() => check(value, inner, path),
        Shape::ScalarOr(_) => check_scalar(value, path),
        Shape::Id => match value.as_str() {
            Some(text) => check_id(text, path),
            None => Err(Refusal::WrongKind(path.to_string())),
        },
        Shape::Map => {
            let map = value
                .as_object()
                .ok_or_else(|| Refusal::WrongKind(path.to_string()))?;
            // The path says "a key", never which: a key could be a name.
            let at = format!("{path}.*");
            map.iter().try_for_each(|(key, v)| {
                check_id(key, &at)?;
                check_scalar(v, &at)
            })
        }
        Shape::List(item) => {
            let items = value
                .as_array()
                .ok_or_else(|| Refusal::WrongKind(path.to_string()))?;
            items
                .iter()
                .enumerate()
                .try_for_each(|(i, v)| check(v, item, &format!("{path}[{i}]")))
        }
        Shape::Object(fields) => {
            let map = value
                .as_object()
                .ok_or_else(|| Refusal::WrongKind(path.to_string()))?;
            map.iter().try_for_each(|(key, v)| {
                let at = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                match fields.iter().find(|(name, _)| name == key) {
                    Some((_, inner)) => check(v, inner, &at),
                    // The key itself is not echoed: it could be a name.
                    None => Err(Refusal::UnknownField(path.to_string())),
                }
            })
        }
    }
}

/// Refuses a bundle with a field outside the allowed set or a text that
/// looks like a BattleTag or an account id.
pub fn validate(bundle: &Value) -> Result<(), Refusal> {
    check(bundle, &BUNDLE, "")
}

/// What the bundle is made from.
pub struct BundleInput<'a> {
    pub session: &'a str,
    pub index: u64,
    pub report: &'a Value,
    /// The parser that wrote the record; None is "unknown version".
    pub parser: Option<&'a ParserStamp>,
    pub app_version: &'a str,
    pub setup: &'a SetupReport,
    pub created_unix: u64,
}

fn config_check(check: &FileCheck) -> Value {
    match check {
        FileCheck::Ok => json!({"state": "ok"}),
        FileCheck::FileMissing => json!({"state": "file_missing"}),
        FileCheck::Unreadable(kind) => json!({"state": "unreadable", "error_kind": kind}),
        FileCheck::SettingsMissing(lines) => {
            json!({"state": "settings_missing", "missing": lines})
        }
    }
}

/// Builds the bundle and runs the check on it. Anything refused means no
/// bundle at all, never a bundle with the field cut out.
pub fn build(input: &BundleInput<'_>) -> Result<Value, Refusal> {
    let now = ParserStamp::current();
    let bundle = json!({
        "format": FORMAT,
        "format_version": FORMAT_VERSION,
        "created_unix": input.created_unix,
        "app_version": input.app_version,
        "parser": input.parser,
        "parser_now": now,
        "game": {
            "session": input.session,
            "index": input.index,
            "build": input.report.get("build"),
        },
        "log_setup": {
            "log_config": config_check(&input.setup.log_config),
            "client_config": config_check(&input.setup.client_config),
        },
        "warnings": input.report.get("warnings"),
        "problems": input.report.get("problems"),
        "report": input.report,
    });
    validate(&bundle)?;
    Ok(bundle)
}

/// The text of the file, as the window shows it and as it is saved.
pub fn to_text(bundle: &Value) -> String {
    let mut text = serde_json::to_string_pretty(bundle).unwrap_or_default();
    text.push('\n');
    text
}

/// File name for a game's bundle; the session name is the log folder's.
pub fn file_name(session: &str, index: u64) -> String {
    let safe: String = session
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!("tavern-ledger-report-{safe}-game{index}.json")
}

/// Writes the text into `dir` under `name`, never over a file already there:
/// a taken name gets `-2`, `-3`… before the extension. Returns the path.
pub fn save_in(dir: &Path, name: &str, text: &str) -> io::Result<PathBuf> {
    let (stem, ext) = name.rsplit_once('.').unwrap_or((name, ""));
    for copy in 1..=MAX_FILE_COPIES {
        let candidate = match (copy, ext) {
            (1, _) => name.to_string(),
            (n, "") => format!("{stem}-{n}"),
            (n, e) => format!("{stem}-{n}.{e}"),
        };
        let path = dir.join(candidate);
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                if let Err(e) = file.write_all(text.as_bytes()) {
                    drop(file);
                    let _ = std::fs::remove_file(&path);
                    return Err(e);
                }
                return Ok(path);
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "too many report files with this name",
    ))
}

/// `%USERPROFILE%\Downloads`, if it exists.
pub fn downloads_dir() -> Option<PathBuf> {
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"))?;
    let dir = PathBuf::from(home).join("Downloads");
    dir.is_dir().then_some(dir)
}

#[cfg(test)]
#[path = "report_bundle_tests.rs"]
mod tests;
