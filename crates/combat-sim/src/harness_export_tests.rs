use super::*;
use crate::harness_duos::Leg;
use crate::harness_misses::Miss;
use crate::model::Minion;

fn band(ids: &[&str]) -> Warband {
    Warband::new(ids.iter().map(|id| Minion::new(*id, 2, 2)).collect())
}

fn solo(player: &[&str], opponent: &[&str], result: CombatResult) -> LoggedCombat {
    LoggedCombat {
        game: 1,
        round: 1,
        opponent_seat: None,
        player: band(player),
        opponent: band(opponent),
        result,
        unknown_gifts: Vec::new(),
    }
}

fn duos(own: &[&str], opponent: &[&str], result: Option<CombatResult>) -> DuosCombat {
    let leg = |ids: &[&str]| Leg {
        seat: None,
        board: band(ids),
    };
    DuosCombat {
        game: 1,
        round: 1,
        own: vec![leg(own)],
        opponent: vec![leg(opponent)],
        local: None,
        teammate: None,
        opponent_seats: Vec::new(),
        out: Vec::new(),
        own_shop: None,
        result,
        unknown_gifts: Vec::new(),
    }
}

fn options() -> Options {
    Options {
        trials: 200,
        ..Options::default()
    }
}

fn none() -> Checked<'static> {
    static NO_BUILDS: BTreeMap<i64, u32> = BTreeMap::new();
    Checked {
        misses: &[],
        replays: &[],
        builds: &NO_BUILDS,
    }
}

fn a_miss(duos: bool) -> Miss {
    Miss {
        game: 7,
        round: 9,
        duos,
        prediction: crate::score::Prediction {
            won: 1.0,
            tied: 0.0,
            lost: 0.0,
        },
        result: CombatResult::Lost,
        coverage: Default::default(),
        inputs: Vec::new(),
    }
}

#[test]
fn counts_blocked_combats_by_card_and_scores_the_rest() {
    // Arrange: every minion is a 2/2, so two beat one.
    let known = ["BGS_119", "BGS_119"];
    let solo = vec![
        solo(&known, &["BG_ICC_026t"], CombatResult::Won),
        solo(&["ZZ_UNKNOWN"], &known, CombatResult::Lost),
        solo(&["ZZ_OTHER", "ZZ_UNKNOWN"], &known, CombatResult::Tie),
    ];
    let duos = vec![
        duos(&known, &["ZZ_UNKNOWN"], Some(CombatResult::Won)),
        duos(&known, &["BG_ICC_026t"], Some(CombatResult::Won)),
        duos(&["ZZ_UNKNOWN"], &known, None),
    ];

    // Act
    let a = accuracy(&solo, &duos, &options());

    // Assert
    assert_eq!(a.solo.with_result, 3);
    assert_eq!(a.solo.blocked, 2);
    assert_eq!(a.solo.report.combats, 1);
    assert_eq!(a.solo.report.top_pick_hits, 1);
    assert_eq!(
        a.duos.with_result, 2,
        "a combat without a result is left out"
    );
    assert_eq!(a.duos.blocked, 1);
    assert_eq!(a.duos.report.combats, 1);
    assert_eq!(
        a.blockers,
        vec![("ZZ_UNKNOWN".to_string(), 3), ("ZZ_OTHER".to_string(), 1)]
    );
    assert!((a.solo.blocked_share() - 2.0 / 3.0).abs() < 1e-9);
}

#[test]
fn hand_summoners_no_longer_block_and_are_scored_apart_too() {
    // Arrange: Expert Aviator plays as a plain 2/2 (its hand summon left out).
    let known = ["BGS_119", "BGS_119"];
    let solo = vec![
        solo(&known, &["BG34_140"], CombatResult::Won),
        solo(&known, &["BG_ICC_026t"], CombatResult::Won),
        solo(&["BG34_140_G"], &["ZZ_UNKNOWN"], CombatResult::Tie),
    ];

    // Act
    let a = accuracy(&solo, &[], &options());

    // Assert
    assert_eq!(a.solo.blocked, 1, "only the card not modelled blocks");
    assert_eq!(a.solo.with_hand_summoner, 2);
    assert_eq!(a.solo.report.combats, 2);
    assert_eq!(a.solo.hand_scored.combats, 1);
    assert_eq!(a.solo.hand_scored.top_pick_hits, 1);
    assert_eq!(a.blockers, vec![("ZZ_UNKNOWN".to_string(), 1)]);
}

#[test]
fn blocked_combats_are_also_played_anyway_and_scored_apart() {
    // Arrange: the blocking cards play as plain 2/2s; two beat one.
    let known = ["BGS_119", "BGS_119"];
    let solo = vec![
        solo(&known, &["ZZ_UNKNOWN"], CombatResult::Won),
        solo(&known, &["ZZ_UNKNOWN"], CombatResult::Lost),
        solo(&known, &["BG_ICC_026t"], CombatResult::Won),
    ];

    // Act
    let a = accuracy(&solo, &[], &options());

    // Assert: blocked combats stay blocked, and are scored apart.
    assert_eq!(a.solo.blocked, 2);
    assert_eq!(a.solo.report.combats, 1);
    assert_eq!(a.solo.anyway.combats, 2);
    assert_eq!(a.solo.anyway.top_pick_hits, 1);
    assert_eq!(a.duos.anyway.combats, 0);
    let text = render_accuracy(&a);
    assert!(
        text.contains("| Solo | blocked, played anyway | 2 |"),
        "{text}"
    );
    assert!(
        text.contains("| Duos | scored, with a hand summoner | 0 | - | - | - |"),
        "{text}"
    );
    // The export keeps its keys: nothing of this goes in it.
    let v: Value =
        serde_json::from_str(&export_json(&a, &options(), "2026-10-11", &none())).unwrap();
    assert!(v["modes"]["solo"].get("anyway").is_none());
    assert!(v["modes"]["solo"].get("hand_scored").is_none());
}

#[test]
fn a_card_twice_on_one_combat_blocks_it_once() {
    let solo = vec![solo(
        &["ZZ_UNKNOWN", "ZZ_UNKNOWN"],
        &["ZZ_UNKNOWN"],
        CombatResult::Won,
    )];
    let a = accuracy(&solo, &[], &options());
    assert_eq!(a.blockers, vec![("ZZ_UNKNOWN".to_string(), 1)]);
    assert_eq!(a.duos.with_result, 0);
}

#[test]
fn export_has_numbers_per_mode_and_no_combat_detail() {
    let solo = vec![
        solo(&["BGS_119", "BGS_119"], &["BG_ICC_026t"], CombatResult::Won),
        solo(&["ZZ_UNKNOWN"], &["BGS_119"], CombatResult::Won),
    ];
    let a = accuracy(&solo, &[], &options());

    let text = export_json(&a, &options(), "2026-10-11", &none());
    let v: Value = serde_json::from_str(&text).unwrap();

    assert!(text.ends_with('\n'));
    assert_eq!(v["format"], FORMAT);
    assert_eq!(v["date"], "2026-10-11");
    assert_eq!(v["engine_version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(v["trials"], 200);
    let s = &v["modes"]["solo"];
    assert_eq!(s["combats_with_result"], 2);
    assert_eq!(s["blocked"], 1);
    assert_eq!(s["blocked_share"], 0.5);
    assert_eq!(s["scored"], 1);
    assert_eq!(s["most_likely_right"], 1);
    assert_eq!(s["mean_p_real"], 1.0);
    assert_eq!(s["calibration"].as_array().unwrap().len(), 5);
    assert_eq!(s["calibration"][4]["predictions"], 1);
    assert_eq!(s["calibration"][2]["mean_said"], Value::Null);
    // No combat at all in Duos: shares are unknown, not zero.
    let d = &v["modes"]["duos"];
    assert_eq!(d["combats_with_result"], 0);
    assert_eq!(d["blocked_share"], Value::Null);
    assert_eq!(d["mean_p_real"], Value::Null);
    assert_eq!(v["blocking_cards"][0]["card_id"], "ZZ_UNKNOWN");
    assert_eq!(v["blocking_cards"][0]["combats"], 1);
    for key in ["game", "round", "seat", "hero", "name", "player_id"] {
        assert!(!text.contains(&format!("\"{key}\"")), "{key} in the export");
    }
}

#[test]
fn the_synthetic_example_in_the_docs_has_the_export_shape() {
    let example = include_str!("../tests/data/accuracy_example.json");
    let v: Value = serde_json::from_str(example).unwrap();
    let misses = vec![a_miss(false)];
    let builds = BTreeMap::from([(253_216, 1)]);
    let checked = Checked {
        misses: &misses,
        replays: &[],
        builds: &builds,
    };
    let ours: Value = serde_json::from_str(&export_json(
        &accuracy(&[], &[], &options()),
        &options(),
        "2026-01-01",
        &checked,
    ))
    .unwrap();
    let keys = |v: &Value| -> Vec<String> {
        let mut k: Vec<String> = v.as_object().unwrap().keys().cloned().collect();
        k.sort();
        k
    };
    assert_eq!(keys(&v), keys(&ours));
    assert_eq!(keys(&v["modes"]["solo"]), keys(&ours["modes"]["solo"]));
    assert_eq!(keys(&v["modes"]["duos"]), keys(&ours["modes"]["duos"]));
    assert_eq!(
        keys(&v["modes"]["solo"]["calibration"][0]),
        keys(&ours["modes"]["solo"]["calibration"][0])
    );
    assert_eq!(keys(&v["misses"]), keys(&ours["misses"]));
    assert_eq!(keys(&v["builds"]), keys(&ours["builds"]));
    assert_eq!(
        keys(&v["misses"]["causes"][0]),
        keys(&ours["misses"]["causes"][0])
    );
    assert_eq!(
        keys(&v["builds"]["checked"][0]),
        keys(&ours["builds"]["checked"][0])
    );
    assert_eq!(v["format"], FORMAT);
}

#[test]
fn renders_the_blocked_combats_by_card() {
    let solo = vec![solo(&["ZZ_UNKNOWN"], &["BGS_119"], CombatResult::Lost)];
    let text = render_accuracy(&accuracy(&solo, &[], &options()));
    assert!(
        text.contains("| Solo | 1 | 1 (100%) | 0 | 0 | 0 | - | - | - |"),
        "{text}"
    );
    assert!(text.contains("`ZZ_UNKNOWN` 1"), "{text}");
}

#[test]
fn dates_from_days_since_1970() {
    assert_eq!(date_of_day(0), "1970-01-01");
    assert_eq!(date_of_day(11_016), "2000-02-29");
    assert_eq!(date_of_day(19_723), "2024-01-01");
    assert_eq!(date_of_day(20_737), "2026-10-11");
    assert_eq!(date_of_day(-1), "1969-12-31");
}

#[test]
fn the_export_has_the_misses_by_cause_with_counts_and_shares() {
    let misses = vec![a_miss(false), a_miss(true), a_miss(true), a_miss(true)];
    let builds = BTreeMap::new();
    let checked = Checked {
        misses: &misses,
        replays: &[],
        builds: &builds,
    };
    let a = accuracy(&[], &[], &options());

    let text = export_json(&a, &options(), "2026-10-12", &checked);
    let v: Value = serde_json::from_str(&text).unwrap();

    assert_eq!(v["misses"]["total"], 4);
    let rows = v["misses"]["causes"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["cause"], "no_replay_exact");
    assert_eq!(rows[0]["solo"], 1);
    assert_eq!(rows[0]["duos"], 3);
    assert_eq!(rows[0]["share"], 1.0);
    // Counts and ids only: nothing of the combat itself.
    assert!(!text.contains("\"round\"") && !text.contains("\"game\""));
}

#[test]
fn no_misses_is_an_empty_list_not_a_missing_key() {
    let a = accuracy(&[], &[], &options());
    let v: Value =
        serde_json::from_str(&export_json(&a, &options(), "2026-10-12", &none())).unwrap();
    assert_eq!(v["misses"]["total"], 0);
    assert_eq!(v["misses"]["causes"], serde_json::json!([]));
}

#[test]
fn the_export_has_the_game_builds_newest_first() {
    let builds = BTreeMap::from([(253_216, 150), (260_001, 3)]);
    let checked = Checked {
        misses: &[],
        replays: &[],
        builds: &builds,
    };
    let a = accuracy(&[], &[], &options());

    let v: Value =
        serde_json::from_str(&export_json(&a, &options(), "2026-10-12", &checked)).unwrap();

    assert_eq!(v["builds"]["newest"], 260_001);
    assert_eq!(v["builds"]["checked"][0]["build"], 260_001);
    assert_eq!(v["builds"]["checked"][0]["count"], 3);
    assert_eq!(v["builds"]["checked"][1]["build"], 253_216);
    let none: Value =
        serde_json::from_str(&export_json(&a, &options(), "2026-10-12", &self::none())).unwrap();
    assert_eq!(none["builds"]["newest"], Value::Null);
}

#[test]
fn only_the_newest_builds_are_exported() {
    let builds: BTreeMap<i64, u32> = (1..=25).map(|b| (250_000 + b, 1)).collect();
    let checked = Checked {
        misses: &[],
        replays: &[],
        builds: &builds,
    };
    let a = accuracy(&[], &[], &options());

    let v: Value =
        serde_json::from_str(&export_json(&a, &options(), "2026-10-12", &checked)).unwrap();

    assert_eq!(v["builds"]["checked"].as_array().unwrap().len(), 20);
    assert_eq!(v["builds"]["newest"], 250_025);
    assert_eq!(v["builds"]["checked"][19]["build"], 250_006);
}

#[test]
fn a_build_that_is_not_a_build_number_is_not_counted() {
    let db = crate::harness::load_cards(include_str!("../tests/data/cards.json")).unwrap();
    let line = |session: &str, build: i64| {
        format!(
            r#"{{"session": "{session}", "index": 1, "saved_at": 1, "report": {{"game_type": "GT_BATTLEGROUNDS", "build": {build}, "rounds": []}}}}"#
        )
    };
    let jsonl = [
        line("a", 0),
        line("b", -5),
        line("c", 9_999_999_999),
        line("d", 253_216),
    ]
    .join(
        "
",
    );

    let h = crate::harness::load_history(&jsonl, &db);

    assert_eq!(h.builds, BTreeMap::from([(253_216, 1)]));
}

#[test]
fn the_history_counts_games_by_build() {
    let db = crate::harness::load_cards(include_str!("../tests/data/cards.json")).unwrap();
    let line = |session: &str, build: &str| {
        format!(
            r#"{{"session": "{session}", "index": 1, "saved_at": 1, "report": {{"game_type": "GT_BATTLEGROUNDS", {build}"rounds": []}}}}"#
        )
    };
    let jsonl = [
        line("a", r#""build": 253216, "#),
        line("b", r#""build": 253216, "#),
        line("c", r#""build": 260001, "#),
        line("d", ""),
    ]
    .join(
        "
",
    );

    let h = crate::harness::load_history(&jsonl, &db);

    assert_eq!(h.builds, BTreeMap::from([(253_216, 2), (260_001, 1)]));
    assert_eq!(h.games, 4);
}
