//! The wrapper on made-up boards (no real card data, D-038): what it
//! accepts, what it refuses, and the C ABI round trip.

use serde_json::{json, Value};

use super::*;

/// A plain tier 1 card with no combat effect, and the Skeleton token.
const PLAIN: &str = "BGS_119";
const SKELETON: &str = "BG_ICC_026t";

fn facts() -> Value {
    json!({ "tier": 1, "tribes": [], "attack": 1, "health": 1, "keywords": [] })
}

fn minion(id: &str, attack: i64, health: i64) -> Value {
    json!({ "card_id": id, "attack": attack, "health": health, "facts": facts() })
}

fn board(minions: Vec<Value>, tier: Option<i64>) -> Value {
    json!({ "tier": tier, "minions": minions })
}

fn request(player: Vec<Value>, opponent: Vec<Value>) -> Value {
    json!({ "player": player, "opponent": opponent, "damage_cap": 15 })
}

fn run(v: &Value) -> Value {
    serde_json::from_str(&run_json(&v.to_string())).expect("the answer is JSON")
}

#[test]
fn a_sure_win_gives_every_trial_and_the_damage() {
    let req = request(
        vec![board(vec![minion(PLAIN, 2, 2)], Some(3))],
        vec![board(vec![minion(SKELETON, 1, 1)], Some(2))],
    );
    let a = run(&req);
    assert_eq!(a["status"], "odds");
    assert_eq!(a["engine"], combat_sim::VERSION);
    assert_eq!(a["trials"], 10_000);
    assert_eq!(
        (a["won"].as_u64(), a["tied"].as_u64(), a["lost"].as_u64()),
        (Some(10_000), Some(0), Some(0))
    );
    // Tier 3 hero plus one tier 1 minion left.
    assert_eq!(a["dealt"], json!({ "low": 4, "likely": 4, "high": 4 }));
    assert_eq!(a["taken"], Value::Null);
    assert_eq!(a["damage_known"], true);
}

#[test]
fn the_damage_cap_applies_and_an_unknown_tier_hides_the_damage() {
    let mut req = request(
        vec![board(vec![minion(PLAIN, 2, 2)], Some(6))],
        vec![board(vec![minion(SKELETON, 1, 1)], None)],
    );
    req["damage_cap"] = json!(5);
    assert_eq!(
        run(&req)["dealt"],
        json!({ "low": 5, "likely": 5, "high": 5 })
    );
    req["player"][0]["tier"] = Value::Null;
    let a = run(&req);
    assert_eq!(a["dealt"], Value::Null);
    assert_eq!(a["damage_known"], false);
}

#[test]
fn the_same_request_gives_the_same_answer() {
    let req = request(
        vec![board(
            vec![minion(PLAIN, 3, 3), minion(PLAIN, 1, 4)],
            Some(2),
        )],
        vec![board(
            vec![minion(PLAIN, 2, 5), minion(SKELETON, 1, 1)],
            Some(2),
        )],
    );
    assert_eq!(run(&req), run(&req));
}

#[test]
fn stats_up_to_the_32_bit_limit_are_played_and_one_more_is_refused() {
    let max = i64::from(i32::MAX);
    for stat in [99_999, 100_000, max] {
        let req = request(
            vec![board(vec![minion(PLAIN, stat, stat)], Some(1))],
            vec![board(vec![minion(SKELETON, 1, 1)], Some(1))],
        );
        let a = run(&req);
        assert_eq!(a["status"], "odds", "{stat}: {a}");
        assert_eq!(a["won"], 10_000, "{stat}");
    }
    // Two minions at the limit trade, with no wrap in between.
    let req = request(
        vec![board(vec![minion(PLAIN, max, max)], Some(1))],
        vec![board(vec![minion(PLAIN, max, max)], Some(1))],
    );
    assert_eq!(run(&req)["tied"], 10_000);
    for (attack, health) in [(max + 1, 1), (1, max + 1)] {
        let req = request(
            vec![board(vec![minion(PLAIN, attack, health)], Some(1))],
            vec![board(vec![minion(SKELETON, 1, 1)], Some(1))],
        );
        assert_eq!(run(&req)["status"], "bad_input", "{attack}/{health}");
    }
}

#[test]
fn a_card_the_engine_does_not_know_is_named_never_guessed() {
    let req = request(
        vec![board(vec![minion("BG_NOT_A_CARD", 2, 2)], Some(1))],
        vec![board(vec![minion(SKELETON, 1, 1)], Some(1))],
    );
    let a = run(&req);
    assert_eq!(a["status"], "not_modelled");
    assert_eq!(a["not_modelled"], json!(["BG_NOT_A_CARD"]));
    assert!(a.get("won").is_none());
}

#[test]
fn a_refused_card_says_why() {
    // Stitched Salvager copies a minion the logged board no longer shows.
    let req = request(
        vec![board(vec![minion("BG31_999", 16, 4)], Some(1))],
        vec![board(vec![minion("BG_NOT_A_CARD", 1, 1)], Some(1))],
    );
    let a = run(&req);
    let reasons = a["reasons"].as_array().unwrap();
    let reason = |id: &str| {
        reasons
            .iter()
            .find(|r| r["card_id"] == id)
            .map(|r| r["reason"].as_str().unwrap().to_string())
            .unwrap_or_default()
    };
    assert!(reason("BG31_999").contains("copies the minion it destroys"));
    assert!(reason("BG_NOT_A_CARD").contains("not reviewed"));
}

#[test]
fn a_card_without_facts_is_named_as_missing() {
    let mut req = request(
        vec![board(vec![minion(PLAIN, 2, 2)], Some(1))],
        vec![board(vec![minion(SKELETON, 1, 1)], Some(1))],
    );
    req["opponent"][0]["minions"][0]["facts"] = Value::Null;
    let a = run(&req);
    assert_eq!(a["status"], "no_card_data");
    assert_eq!(a["missing"], json!([SKELETON]));
}

#[test]
fn approximate_cards_come_with_their_reason() {
    // Round 1: a Deathrattle whose numbers grow during the game.
    let req = request(
        vec![board(vec![minion("BG25_009", 2, 2)], Some(1))],
        vec![board(vec![minion(SKELETON, 1, 1)], Some(1))],
    );
    let a = run(&req);
    assert_eq!(a["status"], "odds");
    let approx = a["approximate"].as_array().unwrap();
    assert_eq!(approx[0]["card_id"], "BG25_009");
    assert!(approx[0]["reason"].as_str().unwrap().contains("grow"));
}

#[test]
fn a_start_of_combat_card_is_flagged() {
    let req = request(
        vec![board(vec![minion("BG26_963_G", 2, 2)], Some(1))],
        vec![board(vec![minion(SKELETON, 1, 1)], Some(1))],
    );
    assert_eq!(run(&req)["start_of_combat"], true);
}

const GOLEMANCY: &str = "BG36_MidGameEffect_000t61";

fn gifted(id: &str, attack: i64, health: i64, gifts: Value) -> Value {
    let mut m = minion(id, attack, health);
    m["gifts"] = gifts;
    m
}

#[test]
fn a_dark_gift_on_a_minion_plays() {
    let opponent = || vec![board(vec![minion(SKELETON, 1, 1)], Some(1))];
    let bare = request(vec![board(vec![minion(PLAIN, 1, 1)], Some(1))], opponent());
    assert_eq!(run(&bare)["tied"], 10_000);
    // Golemancy: the Golem it leaves behind wins the trade.
    let with_gift = request(
        vec![board(
            vec![gifted(PLAIN, 1, 1, json!([GOLEMANCY]))],
            Some(1),
        )],
        opponent(),
    );
    let a = run(&with_gift);
    assert_eq!(
        (a["status"].as_str(), a["won"].as_i64()),
        (Some("odds"), Some(10_000))
    );
}

#[test]
fn a_gift_the_engine_does_not_know_is_named_never_ignored() {
    let req = request(
        vec![board(
            vec![gifted(PLAIN, 2, 2, json!(["BG99_NOT_A_GIFT"]))],
            Some(1),
        )],
        vec![board(vec![minion(SKELETON, 1, 1)], Some(1))],
    );
    let a = run(&req);
    assert_eq!(a["status"], "not_modelled");
    assert_eq!(a["not_modelled"], json!(["BG99_NOT_A_GIFT"]));
}

#[test]
fn a_start_of_combat_gift_is_flagged() {
    let hostility = json!(["BG36_MidGameEffect_000t71"]);
    let req = request(
        vec![board(vec![gifted(PLAIN, 2, 2, hostility)], Some(1))],
        vec![board(vec![minion(SKELETON, 1, 1)], Some(1))],
    );
    assert_eq!(run(&req)["start_of_combat"], true);
}

#[test]
fn bad_gift_lists_are_refused() {
    for gifts in [
        json!("BG36_MidGameEffect_000t61"),
        json!(["a", "b", "c", "d", "e"]),
        json!(["bad id"]),
        json!([3]),
    ] {
        let req = request(
            vec![board(vec![gifted(PLAIN, 1, 1, gifts.clone())], Some(1))],
            vec![board(vec![minion(PLAIN, 1, 1)], Some(1))],
        );
        assert_eq!(run(&req)["status"], "bad_input", "{gifts}");
    }
    let null = request(
        vec![board(vec![gifted(PLAIN, 1, 1, Value::Null)], Some(1))],
        vec![board(vec![minion(PLAIN, 1, 1)], Some(1))],
    );
    assert_eq!(run(&null)["status"], "odds", "null is no gifts");
}

/// A Reborn Undead with no combat effect of its own.
fn reborn_undead() -> Value {
    json!({
        "card_id": PLAIN, "attack": 1, "health": 1, "keywords": ["reborn"],
        "facts": { "tier": 1, "tribes": ["UNDEAD"], "attack": 1, "health": 1, "keywords": [] }
    })
}

#[test]
fn a_boards_game_long_values_play() {
    // T-610: the Undead Attack of the saved round goes to the reborn Undead.
    let opponent = || vec![board(vec![minion(PLAIN, 3, 3)], Some(1))];
    let bare = request(vec![board(vec![reborn_undead()], Some(1))], opponent());
    assert_eq!(
        run(&bare)["lost"],
        10_000,
        "1/1, then 1/1 again: never enough"
    );
    let mut with_values = board(vec![reborn_undead()], Some(1));
    with_values["values"] = json!({ "undead_atk": 5 });
    let a = run(&request(vec![with_values], opponent()));
    assert_eq!(a["status"], "odds");
    assert_eq!(a["lost"], 0, "the reborn 6/1 kills the 3/3: {a}");
}

#[test]
fn a_boards_deity_is_read() {
    // T-671: the board's Deity, as a saved round has it, is accepted.
    let mut b = board(vec![minion(PLAIN, 1, 1)], Some(1));
    b["values"] = json!({
        "deity_dbf": 130_610, "deity_atk": 5, "deity_health": 5, "deity_needed": 4, "deity_progress": 1
    });
    let req = request(vec![b], vec![board(vec![minion(PLAIN, 1, 1)], Some(1))]);
    assert_eq!(run(&req)["status"], "odds");
}

#[test]
fn bad_game_long_values_are_refused() {
    for values in [
        json!([5]),
        json!("5"),
        json!({ "undead_atk": -1 }),
        json!({ "undead_atk": 100_000 }),
        json!({ "undead_atk": "5" }),
        json!({ "tasty_lobster": 1 }),
        json!({ "deity_dbf": 10_000_001 }),
        json!({ "deity_atk": 100_000 }),
    ] {
        let mut b = board(vec![minion(PLAIN, 1, 1)], Some(1));
        b["values"] = values.clone();
        let req = request(vec![b], vec![board(vec![minion(PLAIN, 1, 1)], Some(1))]);
        assert_eq!(run(&req)["status"], "bad_input", "{values}");
    }
    let mut none = board(vec![minion(PLAIN, 1, 1)], Some(1));
    none["values"] = Value::Null;
    let req = request(vec![none], vec![board(vec![minion(PLAIN, 1, 1)], Some(1))]);
    assert_eq!(run(&req)["status"], "odds", "null is none");
}

#[test]
fn duos_teams_bring_two_boards() {
    let req = request(
        vec![
            board(vec![minion(PLAIN, 1, 1)], Some(1)),
            board(vec![minion(PLAIN, 5, 5)], Some(1)),
        ],
        vec![
            board(vec![minion(PLAIN, 1, 1)], Some(1)),
            board(vec![minion(PLAIN, 1, 1)], Some(1)),
        ],
    );
    let a = run(&req);
    assert_eq!(a["status"], "odds");
    assert!(a["won"].as_u64().unwrap() > a["lost"].as_u64().unwrap());
}

#[test]
fn board_keywords_are_read_and_card_text_words_dropped() {
    let mut req = request(
        vec![board(vec![minion(PLAIN, 1, 1)], Some(1))],
        vec![board(vec![minion(PLAIN, 1, 1)], Some(1))],
    );
    req["player"][0]["minions"][0]["keywords"] = json!(["divine_shield", "deathrattle", "rally"]);
    let a = run(&req);
    // A shielded 1/1 against a plain 1/1 always wins.
    assert_eq!(a["won"], 10_000);
    req["player"][0]["minions"][0]["keywords"] = json!(["flying"]);
    assert_eq!(run(&req)["status"], "bad_input");
}

/// One way to spoil a good request.
type Change = Box<dyn Fn(&mut Value)>;

#[test]
fn bad_requests_are_refused_with_a_reason() {
    let ok = request(
        vec![board(vec![minion(PLAIN, 1, 1)], Some(1))],
        vec![board(vec![minion(PLAIN, 1, 1)], Some(1))],
    );
    let cases: Vec<(&str, Change)> = vec![
        ("no player", Box::new(|r| r["player"] = json!([]))),
        (
            "three boards",
            Box::new(|r| {
                r["player"] = json!([r["opponent"][0], r["opponent"][0], r["opponent"][0]])
            }),
        ),
        (
            "bad id",
            Box::new(|r| r["player"][0]["minions"][0]["card_id"] = json!("<b>")),
        ),
        (
            "no health",
            Box::new(|r| r["player"][0]["minions"][0]["health"] = json!(0)),
        ),
        (
            "negative attack",
            Box::new(|r| r["player"][0]["minions"][0]["attack"] = json!(-1)),
        ),
        (
            "fraction",
            Box::new(|r| r["player"][0]["minions"][0]["attack"] = json!(1.5)),
        ),
        (
            "huge",
            Box::new(|r| r["player"][0]["minions"][0]["health"] = json!(1_i64 << 40)),
        ),
        ("tier 8", Box::new(|r| r["player"][0]["tier"] = json!(8))),
        (
            "golden text",
            Box::new(|r| r["player"][0]["minions"][0]["golden"] = json!("yes")),
        ),
        ("cap 0", Box::new(|r| r["damage_cap"] = json!(0))),
        (
            "facts tribe",
            Box::new(|r| r["player"][0]["minions"][0]["facts"]["tribes"] = json!(["<x>"])),
        ),
        (
            "facts health",
            Box::new(|r| r["player"][0]["minions"][0]["facts"]["health"] = json!(0)),
        ),
        (
            "not a list",
            Box::new(|r| r["player"][0]["minions"] = json!({})),
        ),
    ];
    for (name, change) in cases {
        let mut r = ok.clone();
        change(&mut r);
        let a = run(&r);
        assert_eq!(a["status"], "bad_input", "{name}: {a}");
        assert!(a["error"].as_str().is_some_and(|e| !e.is_empty()), "{name}");
    }
    let full = board((0..8).map(|_| minion(PLAIN, 1, 1)).collect(), Some(1));
    let mut r = ok.clone();
    r["player"][0] = full;
    assert_eq!(
        run(&r)["status"],
        "bad_board",
        "eight minions: the engine's own refusal"
    );
    for text in ["", "null", "[]", "{", "\"x\""] {
        assert_eq!(run_json_value(text)["status"], "bad_input", "{text:?}");
    }
    let big = format!("{{\"x\":\"{}\"}}", "a".repeat(input::MAX_INPUT_BYTES));
    assert_eq!(run_json_value(&big)["error"], "the request is too large");
}

fn run_json_value(text: &str) -> Value {
    serde_json::from_str(&run_json(text)).unwrap()
}

#[test]
fn the_c_abi_round_trip_gives_the_same_answer() {
    let req = request(
        vec![board(vec![minion(PLAIN, 2, 2)], Some(1))],
        vec![board(vec![minion(SKELETON, 1, 1)], Some(1))],
    )
    .to_string();
    let ptr = sim_alloc(req.len());
    // SAFETY: `ptr` holds `req.len()` writable bytes from sim_alloc.
    unsafe { std::ptr::copy_nonoverlapping(req.as_ptr(), ptr, req.len()) };
    // SAFETY: as sim_run asks: from sim_alloc with this length, written above.
    let len = unsafe { sim_run(ptr, req.len()) };
    // SAFETY: the answer lives until the next run; `len` bytes long.
    let bytes = unsafe { std::slice::from_raw_parts(sim_answer(), len) };
    assert_eq!(std::str::from_utf8(bytes).unwrap(), run_json(&req));
    // Not UTF-8: refused, not a panic.
    let ptr = sim_alloc(2);
    // SAFETY: two writable bytes from sim_alloc.
    unsafe { std::ptr::copy_nonoverlapping([0xff, 0xfe].as_ptr(), ptr, 2) };
    // SAFETY: as above.
    let len = unsafe { sim_run(ptr, 2) };
    // SAFETY: as above.
    let bytes = unsafe { std::slice::from_raw_parts(sim_answer(), len) };
    assert!(std::str::from_utf8(bytes).unwrap().contains("not UTF-8"));
    // An empty request: an allocation of one byte, read as zero.
    let ptr = sim_alloc(0);
    // SAFETY: from sim_alloc(0), nothing written, length 0.
    let len = unsafe { sim_run(ptr, 0) };
    assert!(len > 0);
}

// ---------- Why each combat went as it did (T-549) ----------

fn saved_minion(id: &str, atk: i64, health: i64, position: i64) -> Value {
    json!({ "card_id": id, "golden": false, "atk": atk, "health": health,
            "position": position, "keywords": [] })
}

fn explain_request(facts_of_plain: Value) -> Value {
    let entries = json!([
        { "side": "own", "player_id": 1, "board": [saved_minion(PLAIN, 3, 3, 1)] },
        { "side": "opponent", "player_id": 2, "board": [saved_minion(PLAIN, 1, 1, 1)] },
    ]);
    let report = json!({
        "game_type": "GT_BATTLEGROUNDS",
        "rounds": [
            { "number": 2, "result": "won", "entries": entries },
            { "number": 3, "result": "lost", "entries": entries },
        ],
    });
    json!({ "explain": { "report": report, "facts": { PLAIN: facts_of_plain } } })
}

#[test]
fn a_saved_game_is_explained_combat_by_combat() {
    let a = run(&explain_request(facts()));
    assert_eq!(a["status"], "explained", "{a}");
    assert_eq!(a["engine"], combat_sim::VERSION);
    let combats = a["combats"].as_array().expect("a list");
    assert_eq!(combats.len(), 2);
    assert_eq!(combats[0]["cause"]["id"], "matched");
    assert_eq!(combats[0]["odds"]["won"], 1.0);
    // The 3/3 always wins; the game says it lost and saved no replay.
    assert_eq!(combats[1]["cause"]["id"], "no_replay_exact");
    assert_eq!(combats[1]["cause"]["replay"], "not_saved");
}

#[test]
fn a_game_build_the_rules_were_not_checked_against_warns_and_still_gives_odds() {
    let mut req = request(
        vec![board(vec![minion(PLAIN, 2, 2)], Some(3))],
        vec![board(vec![minion(SKELETON, 1, 1)], Some(2))],
    );
    let plain = run(&req);
    assert_eq!(plain["new_build"], Value::Null);

    req["build"] = json!(999_999);
    let new = run(&req);
    assert_eq!(new["status"], "odds");
    assert_eq!(new["new_build"], 999_999);
    assert_eq!(new["won"], plain["won"]);
    assert_eq!(new["dealt"], plain["dealt"]);

    for checked in [253_216, 254_477] {
        req["build"] = json!(checked);
        assert_eq!(run(&req)["new_build"], Value::Null, "build {checked}");
    }
    req["build"] = Value::Null;
    assert_eq!(run(&req)["new_build"], Value::Null);
}

#[test]
fn a_bad_build_is_refused() {
    for bad in [
        json!(0),
        json!(-5),
        json!("254477"),
        json!(1.5),
        json!(1e12),
        json!(100_000_000),
    ] {
        let mut req = request(
            vec![board(vec![minion(PLAIN, 1, 1)], Some(1))],
            vec![board(vec![minion(PLAIN, 1, 1)], Some(1))],
        );
        req["build"] = bad.clone();
        assert_eq!(run(&req)["status"], "bad_input", "build {bad}");
    }
}

#[test]
fn a_saved_game_of_a_new_build_is_explained_with_a_warning() {
    let mut req = explain_request(facts());
    assert_eq!(run(&req)["new_build"], Value::Null);
    req["explain"]["report"]["build"] = json!(999_999);
    let a = run(&req);
    assert_eq!(a["status"], "explained", "{a}");
    assert_eq!(a["new_build"], 999_999);
    assert_eq!(a["combats"].as_array().map(Vec::len), Some(2));
    req["explain"]["report"]["build"] = json!(254_477);
    assert_eq!(run(&req)["new_build"], Value::Null);
    // A build out of range is not a build to warn about either.
    req["explain"]["report"]["build"] = json!(100_000_000);
    assert_eq!(run(&req)["new_build"], Value::Null);
    // A build field that is not a number is not a build to warn about.
    req["explain"]["report"]["build"] = json!("x");
    assert_eq!(run(&req)["new_build"], Value::Null);
}

#[test]
fn a_card_without_facts_gives_no_odds_by_name() {
    let a = run(&explain_request(Value::Null));
    assert_eq!(a["combats"][0]["no_odds"], "no_card_data");
    assert_eq!(a["combats"][0]["cards"], json!([PLAIN]));
}

#[test]
fn a_bad_explain_request_is_refused_with_a_reason() {
    let bad_facts = json!({ "explain": { "report": {}, "facts": { "../x": facts() } } });
    assert_eq!(run(&bad_facts)["status"], "bad_input");
    let wrong_facts = json!({ "explain": { "report": {}, "facts": { PLAIN: { "tier": 99 } } } });
    assert_eq!(run(&wrong_facts)["status"], "bad_input");
    let no_report = json!({ "explain": { "report": [], "facts": {} } });
    assert_eq!(run(&no_report)["status"], "bad_input");
    let huge = format!(
        "{{\"explain\": {{\"report\": {{\"pad\": \"{}\"}}, \"facts\": {{}}}}}}",
        "x".repeat(explain::MAX_EXPLAIN_BYTES)
    );
    let a: Value = serde_json::from_str(&run_json(&huge)).unwrap();
    assert_eq!(a["status"], "bad_input");
}
