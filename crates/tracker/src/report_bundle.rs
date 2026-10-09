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
}

const MINION: Shape = Shape::Object(&[
    ("card_id", Shape::Id),
    ("atk", Shape::Scalar),
    ("health", Shape::Scalar),
    ("position", Shape::Scalar),
    ("golden", Shape::Scalar),
]);

const ENTRY: Shape = Shape::Object(&[
    ("side", Shape::Id),
    ("player_id", Shape::Scalar),
    ("hero", Shape::Id),
    ("board", Shape::List(&MINION)),
]);

const ROUND: Shape = Shape::Object(&[
    ("number", Shape::Scalar),
    ("entries", Shape::List(&ENTRY)),
    ("own_health_after", Shape::Scalar),
    ("opponents", Shape::List(&Shape::Scalar)),
    ("health_after", Shape::Map),
]);

const LOBBY_PLAYER: Shape = Shape::Object(&[
    ("player_id", Shape::Scalar),
    ("hero", Shape::Id),
    ("duo_team", Shape::Scalar),
    ("final_place", Shape::Scalar),
    ("final_health", Shape::Scalar),
]);

const TEXTS: Shape = Shape::List(&Shape::Scalar);

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
mod tests {
    use super::*;
    use crate::setup::check_setup;

    const SESSION: &str = "Hearthstone_2026_10_09_15_26_25";

    fn report() -> Value {
        json!({
            "index": 1, "status": "ok", "game_type": "GT_BATTLEGROUNDS", "build": 253216,
            "local_player_id": 3, "hero": "BG20_HERO_202", "teammate_player_id": null,
            "teammate_hero": null, "card_names": {"BG20_HERO_202": "Master Nguyen"},
            "final_place": 2, "final_health": 12,
            "lobby": [{"player_id": 3, "hero": "BG20_HERO_202", "duo_team": null,
                       "final_place": 2, "final_health": 12}],
            "shop_tribes": {"Beast": 4},
            "start_health": {"3": 40, "5": 40},
            "rounds": [{"number": 1, "own_health_after": 40, "opponents": [5],
                "health_after": {"3": 40, "5": 33},
                "entries": [{"side": "own", "player_id": 3, "hero": "BG20_HERO_202",
                    "board": [{"card_id": "BG_X", "atk": 2, "health": 3,
                               "position": 1, "golden": false}]},
                    {"side": "opponent", "player_id": 5, "hero": null, "board": null}]}],
            "warnings": ["a warning"], "problems": [], "not_in_log": ["MMR"],
        })
    }

    fn setup() -> SetupReport {
        check_setup(None, None)
    }

    fn input<'a>(report: &'a Value, setup: &'a SetupReport) -> BundleInput<'a> {
        BundleInput {
            session: SESSION,
            index: 1,
            report,
            parser: None,
            app_version: "0.1.0",
            setup,
            created_unix: 1_790_000_000,
        }
    }

    fn built(report: &Value) -> Result<Value, Refusal> {
        build(&input(report, &setup()))
    }

    // --- what goes in ---

    #[test]
    fn the_bundle_holds_the_report_versions_setup_and_warnings() {
        let report = report();
        let stamp = ParserStamp {
            version: "0.1.0".into(),
            revision: 1,
        };
        let setup = setup();
        let mut i = input(&report, &setup);
        i.parser = Some(&stamp);
        let bundle = build(&i).unwrap();
        assert_eq!(bundle["format"], FORMAT);
        assert_eq!(bundle["app_version"], "0.1.0");
        assert_eq!(bundle["parser"], json!({"version": "0.1.0", "revision": 1}));
        assert_eq!(bundle["parser_now"], json!(ParserStamp::current()));
        assert_eq!(bundle["game"]["build"], 253216);
        assert_eq!(bundle["game"]["session"], SESSION);
        assert_eq!(bundle["warnings"], json!(["a warning"]));
        assert_eq!(bundle["problems"], json!([]));
        assert_eq!(bundle["report"], report);
        assert_eq!(bundle["log_setup"]["log_config"]["state"], "file_missing");
    }

    #[test]
    fn a_record_without_a_parser_stamp_says_unknown_version() {
        let bundle = built(&report()).unwrap();
        assert!(bundle["parser"].is_null());
    }

    #[test]
    fn setup_states_are_words_not_paths() {
        let dir = std::env::temp_dir().join(format!("tl-bundle-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let log = dir.join("log.config");
        std::fs::write(&log, "[Power]\nLogLevel=1\n").unwrap();
        let setup = check_setup(Some(&log), None);
        let bundle = build(&input(&report(), &setup)).unwrap();
        let text = to_text(&bundle);
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(
            bundle["log_setup"]["log_config"]["state"],
            "settings_missing"
        );
        assert_eq!(
            bundle["log_setup"]["log_config"]["missing"],
            json!(["FilePrinting=true", "Verbose=true"])
        );
        assert!(!text.contains("tl-bundle"), "no folder names in the file");
    }

    #[test]
    fn the_text_is_json_that_ends_with_a_newline() {
        let text = to_text(&built(&report()).unwrap());
        assert!(text.ends_with("}\n"));
        let parsed: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["format_version"], FORMAT_VERSION);
    }

    #[test]
    fn a_real_parser_report_passes_the_check() {
        // Every field the parser can write must be in the allowed set.
        let game = bg_parser::report::GameReport::new(
            1,
            bg_parser::report::Status::Ok,
            Some("GT_BATTLEGROUNDS".into()),
            Some(1),
        );
        let report = serde_json::to_value(&game).unwrap();
        assert!(built(&report).is_ok());
    }

    // --- what stays out ---

    #[test]
    fn a_battletag_in_a_warning_is_refused() {
        let mut report = report();
        report["warnings"] = json!(["saw Someone#12345 leave"]);
        let err = built(&report).unwrap_err();
        assert_eq!(err, Refusal::NameShaped("report.warnings[0]".into()));
        assert!(!err.to_string().contains("Someone"), "never echo the value");
    }

    #[test]
    fn battletags_are_refused_in_every_place_a_text_can_be() {
        let tag = "Name#1234";
        let places = [
            |r: &mut Value, t: &str| r["hero"] = json!(t),
            |r: &mut Value, t: &str| r["card_names"] = json!({"X": t}),
            |r: &mut Value, t: &str| r["card_names"] = json!({ t: "x" }),
            |r: &mut Value, t: &str| r["problems"] = json!([t]),
            |r: &mut Value, t: &str| r["rounds"][0]["entries"][0]["hero"] = json!(t),
            |r: &mut Value, t: &str| r["rounds"][0]["entries"][0]["board"][0]["card_id"] = json!(t),
            |r: &mut Value, t: &str| r["shop_tribes"] = json!({ t: 1 }),
        ];
        for place in places {
            let mut r = report();
            place(&mut r, tag);
            assert!(
                matches!(built(&r), Err(Refusal::NameShaped(_))),
                "not refused: {r}"
            );
        }
    }

    #[test]
    fn a_battletag_is_found_inside_a_longer_text_and_with_accents() {
        assert!(looks_like_battletag("player Ñandú#4821 left"));
        assert!(looks_like_battletag("東京#12345"));
        assert!(looks_like_battletag("a#999"));
        assert!(!looks_like_battletag("a#99"));
    }

    #[test]
    fn battletag_look_alikes_are_refused_whatever_comes_before_the_hash() {
        for bad in [
            "Name #1234",
            "Name_#1234",
            "Name!#1234",
            "#12345",
            "Name\u{ff03}1234",
            "Name\u{fe5f}1234",
            "Name#\u{ff11}\u{ff12}\u{ff13}\u{ff14}",
            "\u{e0a}\u{e31}\u{e49}#1234",
        ] {
            assert!(looks_like_battletag(bad), "not refused: {bad:?}");
        }
    }

    #[test]
    fn map_keys_with_a_name_are_refused_without_echoing_the_key() {
        for key in ["Name#1234", "Some Name"] {
            let mut r = report();
            r["card_names"] = json!({ key: "x" });
            let err = built(&r).unwrap_err();
            assert!(!err.to_string().contains("Name"), "{err}");
            assert!(!err.to_string().contains("Bob"), "{err}");
        }
        let mut r = report();
        r["shop_tribes"] = json!({"Bob#4444": {"x": 1}});
        let err = built(&r).unwrap_err();
        assert!(!err.to_string().contains("Bob"), "{err}");
    }

    #[test]
    fn fields_that_are_ids_refuse_free_text_even_without_a_hash() {
        let set: [fn(&mut Value, &str); 6] = [
            |r, t| r["hero"] = json!(t),
            |r, t| r["game_type"] = json!(t),
            |r, t| r["status"] = json!(t),
            |r, t| r["rounds"][0]["entries"][0]["board"][0]["card_id"] = json!(t),
            |r, t| r["shop_tribes"] = json!({ t: 1 }),
            |r, t| r["lobby"][0]["hero"] = json!(t),
        ];
        for put in set {
            for text in ["Some Player", "", "a-b", &"x".repeat(MAX_ID + 1)] {
                let mut r = report();
                put(&mut r, text);
                assert!(built(&r).is_err(), "accepted {text:?}");
            }
        }
    }

    #[test]
    fn a_session_name_with_free_text_is_refused() {
        let report = report();
        let setup = setup();
        let mut i = input(&report, &setup);
        i.session = "Some Player";
        assert!(build(&i).is_err());
    }

    #[test]
    fn card_ids_and_plain_text_are_not_taken_for_battletags() {
        for ok in [
            "BG20_HERO_202_SKIN_B4",
            "TB_BaconShop_HERO_102",
            "round #3 of the game",
            "#1",
            "#12",
            "issue #12",
            "a # b",
            "Master Nguyen",
            "",
        ] {
            assert!(!looks_like_battletag(ok), "wrongly refused: {ok}");
        }
    }

    #[test]
    fn account_ids_are_refused() {
        for bad in [
            "GameAccountId=[hi=1 lo=2]",
            "[hi=144 lo=77]",
            "gameaccountid",
        ] {
            let mut r = report();
            r["warnings"] = json!([bad]);
            assert!(matches!(built(&r), Err(Refusal::NameShaped(_))), "{bad}");
        }
    }

    #[test]
    fn a_field_outside_the_allowed_set_is_refused_whole() {
        let mut r = report();
        r["player_name"] = json!("anything");
        assert_eq!(built(&r), Err(Refusal::UnknownField("report".into())));

        let mut r = report();
        r["lobby"][0]["battletag"] = json!("x");
        assert_eq!(
            built(&r),
            Err(Refusal::UnknownField("report.lobby[0]".into()))
        );

        let mut r = report();
        r["rounds"][0]["entries"][1]["board"] = json!([{"card_id": "A", "owner": "x"}]);
        assert!(matches!(built(&r), Err(Refusal::UnknownField(_))));
    }

    #[test]
    fn an_unknown_field_name_is_never_echoed() {
        let mut r = report();
        r["Secret#9999"] = json!(1);
        let err = built(&r).unwrap_err();
        assert!(!err.to_string().contains("Secret"));
    }

    #[test]
    fn values_of_the_wrong_kind_are_refused() {
        let mut r = report();
        r["hero"] = json!({"name": "x"});
        assert_eq!(built(&r), Err(Refusal::WrongKind("report.hero".into())));
        let mut r = report();
        r["warnings"] = json!("not a list");
        assert!(matches!(built(&r), Err(Refusal::WrongKind(_))));
        assert!(matches!(
            built(&json!("not an object")),
            Err(Refusal::WrongKind(_))
        ));
    }

    #[test]
    fn a_huge_text_is_refused() {
        let mut r = report();
        r["warnings"] = json!(["x".repeat(MAX_TEXT + 1)]);
        assert!(matches!(built(&r), Err(Refusal::TooLong(_))));
    }

    #[test]
    fn validate_is_what_build_runs_so_a_hand_made_bundle_is_checked_too() {
        let mut bundle = built(&report()).unwrap();
        assert!(validate(&bundle).is_ok());
        bundle["app_version"] = json!("me#1234");
        assert!(validate(&bundle).is_err());
    }

    // --- saving ---

    #[test]
    fn file_names_have_only_safe_characters() {
        assert_eq!(
            file_name(SESSION, 2),
            "tavern-ledger-report-Hearthstone_2026_10_09_15_26_25-game2.json"
        );
        assert_eq!(
            file_name("..\\x/y:z", 1),
            "tavern-ledger-report-___x_y_z-game1.json"
        );
    }

    #[test]
    fn saving_never_overwrites_a_file() {
        let dir = std::env::temp_dir().join(format!("tl-save-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let first = save_in(&dir, "r.json", "one").unwrap();
        let second = save_in(&dir, "r.json", "two").unwrap();
        let first_text = std::fs::read_to_string(&first).unwrap();
        let second_text = std::fs::read_to_string(&second).unwrap();
        let second_name = second.file_name().unwrap().to_string_lossy().into_owned();
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(first_text, "one");
        assert_eq!(second_text, "two");
        assert_eq!(second_name, "r-2.json");
    }

    #[test]
    fn saving_into_a_missing_folder_is_an_error() {
        let dir = std::env::temp_dir().join("tl-no-such-folder-017");
        assert!(save_in(&dir, "r.json", "x").is_err());
    }
}
