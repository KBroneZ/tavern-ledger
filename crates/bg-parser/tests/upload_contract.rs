//! The parser's report and upload-game's validator must agree on every key
//! (T-104h, D-047). `supabase/functions/upload-game/report_keys.json` lists
//! them per level; this test serializes every game in the fixtures (real and
//! synthetic) and fails when the parser writes a key the list lacks, or the
//! list keeps a key the parser no longer writes. The validator imports the
//! same list and will not start if it checks a different set; CI does not run
//! Deno, so this test also reads that table from validate.ts and compares it.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn contract() -> BTreeMap<String, BTreeSet<String>> {
    let path = root().join("supabase/functions/upload-game/report_keys.json");
    let text = fs::read_to_string(&path).expect("report_keys.json");
    let value: Value = serde_json::from_str(&text).expect("valid json");
    serde_json::from_value(value["keys"].clone()).expect("keys: level -> [key]")
}

/// Every report the parser gives for the fixture logs, as JSON.
fn reports() -> Vec<(String, Value)> {
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data");
    let mut out = Vec::new();
    for dir in [data.clone(), data.join("real")] {
        let mut logs: Vec<PathBuf> = fs::read_dir(&dir)
            .expect("fixture dir")
            .map(|e| e.expect("dir entry").path())
            .filter(|p| p.extension().is_some_and(|x| x == "log"))
            .collect();
        logs.sort();
        for log in logs {
            let text = fs::read_to_string(&log).expect("fixture");
            let reports = bg_parser::parse_reader(text.as_bytes()).expect("in-memory read");
            let name = log.file_name().unwrap().to_string_lossy().into_owned();
            for r in reports {
                out.push((
                    name.clone(),
                    serde_json::to_value(&r).expect("serializable"),
                ));
            }
        }
    }
    assert!(out.len() > 10, "fixtures found");
    out
}

/// Adds the keys of every object at `level` under `value` to `seen`.
fn walk(level: &str, value: &Value, seen: &mut BTreeMap<String, BTreeSet<String>>) {
    let Some(obj) = value.as_object() else {
        return;
    };
    seen.entry(level.to_string())
        .or_default()
        .extend(obj.keys().cloned());
    let children = [
        ("report", "lobby", "report.lobby[]"),
        ("report", "rounds", "report.rounds[]"),
        ("report.rounds[]", "entries", "report.rounds[].entries[]"),
        (
            "report.rounds[].entries[]",
            "board",
            "report.rounds[].entries[].board[]",
        ),
        ("report.shop", "turns", "report.shop.turns[]"),
        (
            "report.shop.turns[]",
            "offers",
            "report.shop.turns[].offers[]",
        ),
        ("report.shop", "tier_ups", "report.shop.tier_ups[]"),
        ("report.shop", "actions", "report.shop.actions[]"),
    ];
    for (parent, key, child) in children {
        if parent == level {
            for item in obj.get(key).and_then(Value::as_array).into_iter().flatten() {
                walk(child, item, seen);
            }
        }
    }
    if level == "report" {
        if let Some(shop) = obj.get("shop") {
            walk("report.shop", shop, seen);
        }
    }
}

#[test]
fn the_parser_writes_exactly_the_keys_upload_game_checks() {
    let mut seen = BTreeMap::new();
    for (_, report) in reports() {
        walk("report", &report, &mut seen);
    }
    let contract = contract();
    for (level, keys) in &seen {
        let listed = contract
            .get(level)
            .unwrap_or_else(|| panic!("{level} is not in report_keys.json"));
        let new: Vec<_> = keys.difference(listed).collect();
        assert!(
            new.is_empty(),
            "the parser writes {level} keys that upload-game does not know: {new:?}. \
             Add them to supabase/functions/upload-game/report_keys.json and check them \
             in validate.ts (and bump VALIDATOR_VERSION there), or uploads will be refused."
        );
        let gone: Vec<_> = listed.difference(keys).collect();
        assert!(
            gone.is_empty(),
            "report_keys.json lists {level} keys the parser no longer writes: {gone:?}"
        );
    }
}

/// The `KEYS` table in validate.ts, level by level: a quoted string outside
/// brackets names a level, quoted strings inside its brackets are its keys.
fn validator_keys() -> BTreeMap<String, BTreeSet<String>> {
    let source =
        fs::read_to_string(root().join("supabase/functions/upload-game/validate.ts")).unwrap();
    let start = source
        .find("const KEYS")
        .expect("KEYS table in validate.ts");
    let rest = &source[start..];
    let end = rest.find("\n};").expect("end of KEYS");
    let body = &rest[rest.find('{').unwrap() + 1..end];
    let mut table: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let (mut level, mut inside) = (String::new(), false);
    let mut parts = body.split('"');
    let mut between = parts.next().unwrap_or("");
    while let Some(text) = parts.next() {
        // `between` is what came before this quoted string.
        inside = match (between.rfind('['), between.rfind(']')) {
            (Some(open), Some(close)) => open > close,
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (None, None) => inside,
        };
        if inside {
            table.get_mut(&level).unwrap().insert(text.to_string());
        } else {
            level = text.to_string();
            table.insert(level.clone(), BTreeSet::new());
        }
        between = parts.next().unwrap_or("");
    }
    table
}

#[test]
fn validate_ts_checks_exactly_the_listed_keys() {
    let checked = validator_keys();
    assert!(checked["report"].contains("start_health"), "read the table");
    assert_eq!(
        checked,
        contract(),
        "the KEYS table in validate.ts differs from report_keys.json \
         (the function would refuse to start)"
    );
}

/// Every player id a report mentions outside the health maps.
fn player_ids(report: &Value) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    let mut add = |v: &Value| {
        if let Some(n) = v.as_i64() {
            ids.insert(n.to_string());
        }
    };
    add(&report["local_player_id"]);
    add(&report["teammate_player_id"]);
    for p in report["lobby"].as_array().into_iter().flatten() {
        add(&p["player_id"]);
    }
    for r in report["rounds"].as_array().into_iter().flatten() {
        for e in r["entries"].as_array().into_iter().flatten() {
            add(&e["player_id"]);
        }
    }
    ids
}

/// What validate.ts accepts in `start_health` and `rounds[].health_after`.
#[test]
fn health_maps_keep_to_what_upload_game_accepts() {
    for (name, report) in reports() {
        let ids = player_ids(&report);
        let rounds = report["rounds"].as_array().cloned().unwrap_or_default();
        let maps = std::iter::once(&report["start_health"])
            .chain(rounds.iter().map(|r| &r["health_after"]));
        for map in maps {
            let map = map.as_object().expect("an object");
            assert!(map.len() <= 16, "{name}: {} entries", map.len());
            for (pid, health) in map {
                assert!(ids.contains(pid), "{name}: player {pid} not in the report");
                let health = health.as_i64().expect("an integer");
                assert!((0..=100_000).contains(&health), "{name}: health {health}");
            }
        }
    }
}
