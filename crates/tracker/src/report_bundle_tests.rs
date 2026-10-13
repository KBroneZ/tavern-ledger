//! Tests of the problem report bundle (T-110): what goes in and what stays out.

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
        app_log: &[],
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
    // And a full shop record (T-204, T-205), and the revision 4 fields:
    // results, hero pick, skin links, shop extras (T-206 to T-214).
    let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../bg-parser/tests/data");
    for name in [
        "solo_shop_game.log",
        "solo_data_game.log",
        "duo_data_game.log",
        "real/b253216_duos.log",
        "real/b253216_duos_gifts.log",
        "real/b253216_duos_rally.log",
    ] {
        let log = std::fs::read(data.join(name)).unwrap();
        let games = bg_parser::parse_reader(log.as_slice()).unwrap();
        let report = serde_json::to_value(&games[0]).unwrap();
        assert!(!report["shop"]["turns"].as_array().unwrap().is_empty());
        assert!(built(&report).is_ok(), "{name}: {:?}", built(&report).err());
    }
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
        |r: &mut Value, t: &str| {
            r["rounds"][0]["replay"] =
                json!({"minions": [{"card_id": t}], "start": 1, "events": [], "complete": true})
        },
        |r: &mut Value, t: &str| {
            r["rounds"][0]["replay"] = json!({"minions": [], "start": 0,
                "events": [{"kind": "board_in", "side": "own", "hero": t}], "complete": true})
        },
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

// --- the app's error log (T-580) ---

fn log_line(n: usize) -> String {
    format!("2026-10-13T09:05:00Z INFO game game {n} saved (status ok)")
}

#[test]
fn the_bundle_carries_the_last_log_lines_and_none_without_a_log() {
    let report = report();
    let setup = setup();
    assert_eq!(
        build(&input(&report, &setup)).unwrap()["app_log"],
        json!([])
    );
    let lines = vec![log_line(1), log_line(2)];
    let mut i = input(&report, &setup);
    i.app_log = &lines;
    assert_eq!(build(&i).unwrap()["app_log"], json!(lines));
}

#[test]
fn only_the_newest_log_lines_go_in() {
    let report = report();
    let setup = setup();
    let lines: Vec<String> = (0..MAX_LOG_LINES + 50).map(log_line).collect();
    let mut i = input(&report, &setup);
    i.app_log = &lines;
    let bundle = build(&i).unwrap();
    let kept = bundle["app_log"].as_array().unwrap();
    assert_eq!(kept.len(), MAX_LOG_LINES);
    assert_eq!(kept.last().unwrap(), &json!(lines.last().unwrap()));
    assert_eq!(kept[0], json!(lines[50]));
}

#[test]
fn a_log_line_that_looks_like_a_name_is_left_out_and_the_report_still_builds() {
    let report = report();
    let setup = setup();
    let lines = vec![
        log_line(1),
        "2026-10-13T09:05:00Z WARN app seen Jaina#1234 here".to_string(),
        "2026-10-13T09:05:00Z WARN app GameAccountId=[hi=1 lo=2]".to_string(),
        log_line(2),
    ];
    let mut i = input(&report, &setup);
    i.app_log = &lines;
    let text = to_text(&build(&i).unwrap());
    assert!(!text.contains("Jaina"));
    assert!(!text.contains("GameAccountId"));
    assert!(text.contains("game 1 saved") && text.contains("game 2 saved"));
}
