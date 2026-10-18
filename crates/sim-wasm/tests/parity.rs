//! The website's odds are the app's odds (T-606): each made-up request in
//! `data/parity.json` is played by this crate and by the app's own odds code
//! (`tracker::odds::run_odds`, the overlay's), and the two must agree. The
//! file also keeps this crate's answers: the website's tests run the built
//! `.wasm` on the same requests and must get the same answers
//! (`web/test/unit/simEngine.test.ts`). Run with `SIM_PARITY_WRITE=1` to
//! write the answers after an engine change.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use bg_parser::report::{CombatValues, Minion as LogMinion};
use combat_sim::{BaseCard, Keywords, Tribes};
use serde_json::{json, Value};
use tracker::odds::{run_odds, DuosInput, Facts, OddsInput, Seen};

/// Saved keyword names as `&'static str`, as the parser's minions hold them.
const SAVED: [&str; 8] = [
    "taunt",
    "divine_shield",
    "reborn",
    "venomous",
    "windfury",
    "stealth",
    "deathrattle",
    "rally",
];
const SIM_ONLY: [&str; 2] = ["poisonous", "mega_windfury"];

fn path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/parity.json")
}

fn facts(tier: i64, tribes: &[&str], attack: i64, health: i64, keywords: &[&str]) -> Value {
    json!({ "tier": tier, "tribes": tribes, "attack": attack, "health": health, "keywords": keywords })
}

fn m(id: &str, attack: i64, health: i64, keywords: &[&str], f: Value) -> Value {
    json!({ "card_id": id, "attack": attack, "health": health, "golden": id.ends_with("_G"), "keywords": keywords, "facts": f })
}

fn board(tier: Option<i64>, minions: Vec<Value>) -> Value {
    json!({ "tier": tier, "minions": minions })
}

/// The requests: Solo and Duos, Deathrattles, keywords, an approximate card,
/// a start-of-combat card, an unknown tier, a card not modelled and one
/// without facts. Card ids are real (the engine keys its rules on them); the
/// facts are made up.
fn requests() -> Vec<(&'static str, Value)> {
    let plain = |a, h| facts(1, &[], a, h, &[]);
    let skeleton = || facts(1, &["UNDEAD"], 1, 1, &[]);
    let solo = |p: Value, o: Value| json!({ "player": [p], "opponent": [o], "damage_cap": 15 });
    vec![
        (
            "solo plain",
            solo(
                board(
                    Some(3),
                    vec![
                        m("BGS_119", 3, 4, &[], plain(1, 2)),
                        m("BGS_119", 2, 2, &["taunt"], plain(1, 2)),
                    ],
                ),
                board(
                    Some(2),
                    vec![
                        m("BG_ICC_026t", 1, 1, &[], skeleton()),
                        m("BGS_119", 4, 3, &["divine_shield"], plain(1, 2)),
                    ],
                ),
            ),
        ),
        (
            "solo deathrattles and keywords",
            solo(
                board(
                    Some(4),
                    vec![
                        m(
                            "BG28_300",
                            4,
                            6,
                            &["reborn"],
                            facts(3, &["UNDEAD"], 2, 2, &["REBORN"]),
                        ),
                        m(
                            "BG29_611",
                            3,
                            3,
                            &["venomous"],
                            facts(2, &["MECHANICAL"], 3, 3, &[]),
                        ),
                        m("BGS_119", 7, 2, &["windfury", "stealth"], plain(1, 2)),
                    ],
                ),
                board(
                    Some(5),
                    vec![
                        m(
                            "BG30_125_G",
                            6,
                            8,
                            &["deathrattle"],
                            facts(4, &["UNDEAD"], 4, 6, &[]),
                        ),
                        m("BGS_119", 5, 9, &["poisonous", "taunt"], plain(1, 2)),
                        m("BG_ICC_026t", 2, 2, &[], skeleton()),
                    ],
                ),
            ),
        ),
        (
            "solo approximate and start of combat",
            solo(
                board(
                    Some(3),
                    vec![
                        m("BG25_009", 3, 3, &[], facts(2, &["QUILBOAR"], 2, 2, &[])),
                        m("BG26_963_G", 4, 4, &[], facts(3, &["DRAGON"], 2, 2, &[])),
                    ],
                ),
                board(
                    Some(3),
                    vec![
                        m("BGS_119", 4, 4, &["mega_windfury"], plain(1, 2)),
                        m("BGS_119", 3, 5, &[], plain(1, 2)),
                    ],
                ),
            ),
        ),
        (
            "solo start-of-combat attack and Blood Gems",
            solo(
                board(
                    Some(5),
                    vec![
                        m("BG33_430", 2, 5, &[], facts(3, &["QUILBOAR"], 2, 5, &[])),
                        m(
                            "BG33_886",
                            3,
                            4,
                            &["rally"],
                            facts(1, &["QUILBOAR"], 2, 3, &[]),
                        ),
                    ],
                ),
                board(
                    Some(6),
                    vec![
                        m(
                            "BG36_849",
                            7,
                            7,
                            &["rally"],
                            facts(6, &["DRAGON"], 7, 7, &[]),
                        ),
                        m("BGS_119", 3, 5, &[], plain(1, 2)),
                    ],
                ),
            ),
        ),
        (
            "solo unknown tier",
            solo(
                board(
                    None,
                    vec![
                        m("BGS_119", 3, 3, &[], plain(1, 2)),
                        m("BGS_119", 2, 1, &[], plain(1, 2)),
                    ],
                ),
                board(
                    Some(2),
                    vec![
                        m("BGS_119", 3, 3, &[], plain(1, 2)),
                        m("BGS_119", 1, 2, &[], plain(1, 2)),
                    ],
                ),
            ),
        ),
        (
            "solo not modelled",
            solo(
                board(Some(1), vec![m("BG_NOT_A_CARD", 3, 3, &[], plain(1, 2))]),
                board(Some(1), vec![m("BGS_119", 3, 3, &[], plain(1, 2))]),
            ),
        ),
        (
            "solo card data missing",
            solo(
                board(Some(1), vec![m("BG_ICC_026t", 3, 3, &[], Value::Null)]),
                board(Some(1), vec![m("BGS_119", 3, 3, &[], plain(1, 2))]),
            ),
        ),
        (
            "duos two boards each",
            json!({
                "player": [
                    board(Some(4), vec![m("BGS_119", 3, 3, &[], plain(1, 2)), m("BG28_300", 2, 2, &[], facts(3, &["UNDEAD"], 2, 2, &[]))]),
                    board(Some(3), vec![m("BGS_119", 3, 2, &["divine_shield"], plain(1, 2))]),
                ],
                "opponent": [
                    board(Some(5), vec![m("BGS_119", 5, 4, &[], plain(1, 2)), m("BG_ICC_026t", 1, 1, &[], skeleton())]),
                    board(Some(2), vec![m("BG29_611", 4, 5, &["taunt"], facts(2, &["MECHANICAL"], 3, 3, &[]))]),
                ],
                "damage_cap": 15
            }),
        ),
        (
            // T-610: each board's game-long values from the saved round.
            "solo game-long values",
            solo(
                with_values(
                    board(
                        Some(5),
                        vec![
                            m("BG33_430", 2, 5, &[], facts(3, &["QUILBOAR"], 2, 5, &[])),
                            m(
                                "BG33_886",
                                3,
                                4,
                                &["rally"],
                                facts(1, &["QUILBOAR"], 2, 3, &[]),
                            ),
                        ],
                    ),
                    json!({ "blood_gem_atk": 3, "blood_gem_health": 2 }),
                ),
                with_values(
                    board(
                        Some(4),
                        vec![
                            m(
                                "BG28_300",
                                4,
                                3,
                                &["reborn"],
                                facts(3, &["UNDEAD"], 2, 2, &[]),
                            ),
                            m("BGS_119", 3, 5, &[], plain(1, 2)),
                        ],
                    ),
                    json!({ "undead_atk": 4 }),
                ),
            ),
        ),
        (
            "duos game-long values",
            json!({
                "player": [
                    board(Some(4), vec![m("BGS_119", 3, 3, &[], plain(1, 2))]),
                    with_values(
                        board(Some(3), vec![m("BG28_300", 2, 2, &["reborn"], facts(3, &["UNDEAD"], 2, 2, &[]))]),
                        json!({ "undead_atk": 6 }),
                    ),
                ],
                "opponent": [
                    board(Some(5), vec![m("BGS_119", 5, 4, &[], plain(1, 2))]),
                    with_values(
                        board(Some(2), vec![m("BG_ICC_026t", 1, 1, &["reborn"], skeleton())]),
                        json!({ "undead_atk": 2 }),
                    ),
                ],
                "damage_cap": 15
            }),
        ),
        (
            "new game build",
            json!({
                "player": [board(Some(3), vec![m("BGS_119", 3, 4, &[], plain(1, 2))])],
                "opponent": [board(Some(2), vec![m("BGS_119", 2, 2, &[], plain(1, 2))])],
                "damage_cap": 15,
                "build": 999_999
            }),
        ),
    ]
}

fn with_values(mut board: Value, values: Value) -> Value {
    board["values"] = values;
    board
}

fn statics(names: &[Value], list: &[&'static str]) -> Vec<&'static str> {
    names
        .iter()
        .filter_map(|n| list.iter().find(|k| Some(**k) == n.as_str()).copied())
        .collect()
}

fn log_board(b: &Value) -> Vec<LogMinion> {
    let minions = b["minions"].as_array().unwrap();
    minions
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let words = m["keywords"].as_array().unwrap();
            LogMinion {
                card_id: Some(m["card_id"].as_str().unwrap().to_string()),
                atk: m["attack"].as_i64().unwrap(),
                health: m["health"].as_i64().unwrap(),
                position: i as i64 + 1,
                golden: m["golden"].as_bool().unwrap(),
                keywords: statics(words, &SAVED),
                sim_keywords: statics(words, &SIM_ONLY),
                gifts: m["gifts"]
                    .as_array()
                    .map(|g| statics(g, &bg_parser::gifts::GIFT_IDS))
                    .unwrap_or_default(),
                gift_unknown: false,
                buffs: Vec::new(),
                counters: [0, 0],
            }
        })
        .collect()
}

fn fact_table(req: &Value) -> BTreeMap<String, Facts> {
    let mut out = BTreeMap::new();
    let boards = req["player"]
        .as_array()
        .unwrap()
        .iter()
        .chain(req["opponent"].as_array().unwrap());
    for m in boards.flat_map(|b| b["minions"].as_array().unwrap()) {
        let f = &m["facts"];
        if f.is_null() {
            continue;
        }
        let names = |k: &str| -> Vec<String> {
            f[k].as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap().to_string())
                .collect()
        };
        let facts = Facts::from_names(
            f["attack"].as_i64().unwrap() as i32,
            f["health"].as_i64().unwrap() as i32,
            f["tier"].as_i64().map(|t| t as u8),
            &names("tribes"),
            &names("keywords"),
        );
        out.insert(m["card_id"].as_str().unwrap().to_string(), facts);
    }
    out
}

/// A board's game-long values by the request's (and the saved report's) names.
fn values(b: &Value) -> CombatValues {
    let n = |name: &str| b["values"][name].as_i64().unwrap_or(0);
    CombatValues {
        blood_gem: (n("blood_gem_atk"), n("blood_gem_health")),
        spell: (n("spell_atk"), n("spell_health")),
        knights_dead: n("knights_dead"),
        undead_attack: n("undead_atk"),
        beetle: (n("beetle_atk"), n("beetle_health")),
    }
}

fn seen(b: &Value) -> Seen {
    Seen {
        seat: 9,
        round: 1,
        board: log_board(b),
        tier: b["tier"].as_i64(),
        values: values(b),
        kit: None,
    }
}

/// The request as the overlay's input: the player's first board is their
/// own, in Duos the second is the teammate's and the player fights first.
fn app_input(req: &Value) -> OddsInput {
    let (p, o) = (
        req["player"].as_array().unwrap(),
        req["opponent"].as_array().unwrap(),
    );
    OddsInput {
        opponent_seat: 3,
        round: 1,
        seen_after_start_of_combat: false,
        own: log_board(&p[0]),
        opponent: log_board(&o[0]),
        own_values: values(&p[0]),
        opponent_values: values(&o[0]),
        own_kit: None,
        opponent_kit: None,
        estimate: false,
        own_tier: p[0]["tier"].as_i64(),
        opponent_tier: o[0]["tier"].as_i64(),
        damage_cap: req["damage_cap"].as_i64(),
        duos: (p.len() == 2).then(|| DuosInput {
            local_first: true,
            teammate: seen(&p[1]),
            opponent_teammate: seen(&o[1]),
        }),
        build: req["build"].as_i64(),
    }
}

fn share(answer: &Value, key: &str) -> f64 {
    answer[key].as_f64().unwrap() / answer["trials"].as_f64().unwrap()
}

fn ids(list: &Value) -> Vec<String> {
    list.as_array()
        .map(|l| {
            l.iter()
                .map(|x| x["card_id"].as_str().or(x.as_str()).unwrap().to_string())
                .collect()
        })
        .unwrap_or_default()
}

/// The app's answer and this crate's agree on everything the page shows.
fn assert_same_as_app(name: &str, req: &Value, answer: &Value) {
    let table = fact_table(req);
    let lookup = |id: &str| table.get(id).copied();
    let app = run_odds(&app_input(req), &lookup, Duration::from_secs(3600));
    let spread = |s: Option<tracker::odds::Spread>| {
        s.map_or(
            Value::Null,
            |s| json!({ "low": s.low, "likely": s.likely, "high": s.high }),
        )
    };
    match answer["status"].as_str().unwrap() {
        "odds" => {
            assert_eq!(app.gap, None, "{name}");
            assert_eq!(
                answer["trials"].as_u64(),
                Some(u64::from(app.trials)),
                "{name}"
            );
            assert_eq!(share(answer, "won"), app.won, "{name}");
            assert_eq!(share(answer, "tied"), app.tied, "{name}");
            assert_eq!(share(answer, "lost"), app.lost, "{name}");
            assert_eq!(answer["dealt"], spread(app.dealt), "{name}");
            assert_eq!(answer["taken"], spread(app.taken), "{name}");
            assert_eq!(ids(&answer["approximate"]), app.approximate, "{name}");
            assert_eq!(
                answer["start_of_combat"].as_bool(),
                Some(app.start_of_combat),
                "{name}"
            );
            assert_eq!(answer["new_build"].as_i64(), app.new_build, "{name}");
        }
        "not_modelled" => {
            assert_eq!(app.gap, Some(tracker::odds::Gap::NotModelled), "{name}");
            assert_eq!(ids(&answer["not_modelled"]), app.missing, "{name}");
        }
        "no_card_data" => {
            assert_eq!(app.gap, Some(tracker::odds::Gap::NoCardData), "{name}");
            assert_eq!(ids(&answer["missing"]), app.missing, "{name}");
        }
        other => panic!("{name}: unexpected {other}"),
    }
}

#[test]
fn the_website_gives_the_apps_odds_on_the_same_boards() {
    let cases: Vec<Value> = requests()
        .into_iter()
        .map(|(name, request)| {
            let answer: Value =
                serde_json::from_str(&sim_wasm::run_json(&request.to_string())).unwrap();
            assert_same_as_app(name, &request, &answer);
            json!({ "name": name, "request": request, "answer": answer })
        })
        .collect();
    let file = path();
    if std::env::var_os("SIM_PARITY_WRITE").is_some() {
        let text = serde_json::to_string_pretty(&json!({ "cases": cases })).unwrap();
        std::fs::write(&file, text + "\n").unwrap();
        return;
    }
    let saved: Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert_eq!(
        saved["cases"],
        json!(cases),
        "run with SIM_PARITY_WRITE=1 after an engine change, then rebuild the .wasm"
    );
}

#[test]
fn every_status_is_in_the_file() {
    let statuses: Vec<String> = requests()
        .into_iter()
        .map(|(_, r)| {
            serde_json::from_str::<Value>(&sim_wasm::run_json(&r.to_string())).unwrap()["status"]
                .to_string()
        })
        .collect();
    for s in ["odds", "not_modelled", "no_card_data"] {
        assert!(statuses.iter().any(|x| x.trim_matches('"') == s), "{s}");
    }
    // Both the engine and the facts word lists agree with the app's.
    assert_eq!(
        Keywords::from_name("POISONOUS"),
        Keywords::from_name("poisonous")
    );
    assert!(Tribes::from_name("UNDEAD").is_some());
    let _ = BaseCard {
        attack: 0,
        health: 1,
        keywords: Keywords::NONE,
    };
}
