use super::*;
use crate::harness_duos::Leg;
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
    }
}

fn options() -> Options {
    Options {
        trials: 200,
        ..Options::default()
    }
}

#[test]
fn counts_blocked_combats_by_card_and_scores_the_rest() {
    // Arrange: every minion is a 2/2, so two beat one.
    let known = ["BGS_119", "BGS_119"];
    let solo = vec![
        solo(&known, &["BG_ICC_026t"], CombatResult::Won),
        solo(&["BG34_140"], &known, CombatResult::Lost),
        solo(&["BG34_140", "ZZ_UNKNOWN"], &known, CombatResult::Tie),
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
    assert_eq!(a.solo.with_hand_summoner, 2);
    assert_eq!(a.solo.blocked_by_hand_only, 1);
    assert_eq!(a.solo.report.combats, 1);
    assert_eq!(a.solo.report.top_pick_hits, 1);
    assert_eq!(
        a.duos.with_result, 2,
        "a combat without a result is left out"
    );
    assert_eq!(a.duos.blocked, 1);
    assert_eq!(a.duos.blocked_by_hand_only, 0);
    assert_eq!(a.duos.report.combats, 1);
    assert_eq!(
        a.blockers,
        vec![("BG34_140".to_string(), 2), ("ZZ_UNKNOWN".to_string(), 2)]
    );
    assert!((a.solo.blocked_share() - 2.0 / 3.0).abs() < 1e-9);
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

    let text = export_json(&a, &options(), "2026-10-11");
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
    let ours: Value = serde_json::from_str(&export_json(
        &accuracy(&[], &[], &options()),
        &options(),
        "2026-01-01",
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
    assert_eq!(v["format"], FORMAT);
}

#[test]
fn renders_the_blocked_combats_by_card() {
    let solo = vec![solo(&["BG34_140"], &["BGS_119"], CombatResult::Lost)];
    let text = render_accuracy(&accuracy(&solo, &[], &options()));
    assert!(
        text.contains("| Solo | 1 | 1 (100%) | 1 | 1 | 0 | 0 | - | - | - |"),
        "{text}"
    );
    assert!(text.contains("`BG34_140` 1"), "{text}");
}

#[test]
fn dates_from_days_since_1970() {
    assert_eq!(date_of_day(0), "1970-01-01");
    assert_eq!(date_of_day(11_016), "2000-02-29");
    assert_eq!(date_of_day(19_723), "2024-01-01");
    assert_eq!(date_of_day(20_737), "2026-10-11");
    assert_eq!(date_of_day(-1), "1969-12-31");
}
